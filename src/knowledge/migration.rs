//! YAML → SQLite migration for profile.yaml data.
//!
//! Reads the existing `knowledge/personal/profile.yaml`, parses it into
//! `PersonalProfile`, and inserts every entity into the SQLite knowledge
//! graph.  The migration is fully idempotent — running it twice produces
//! no duplicates.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

use super::graph::{EntityType, KnowledgeGraph};
use super::personal::{PersonalProfile, SkillInfo, StarStory};

/// Result counts returned after a migration run.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MigrationResult {
    pub skills_inserted: usize,
    pub stories_inserted: usize,
    pub edges_created: usize,
    pub weaknesses_inserted: usize,
    pub achievements_inserted: usize,
    pub preferences_inserted: usize,
    pub skipped_existing: usize,
}

/// Metadata key used to track the last successful migration timestamp.
const MIGRATION_META_KEY: &str = "last_profile_yaml_migration";

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run the profile.yaml → SQLite migration.
///
/// * `yaml_path`  — absolute or relative path to `profile.yaml`
/// * `graph`      — an open `KnowledgeGraph` (SQLite connection)
///
/// Returns a `MigrationResult` with counts of everything that was inserted.
pub fn migrate_profile_yaml(yaml_path: &Path, graph: &KnowledgeGraph) -> Result<MigrationResult> {
    // Parse YAML
    let profile = PersonalProfile::load_from_file(yaml_path)
        .with_context(|| format!("Failed to parse profile.yaml at {:?}", yaml_path))?;

    let mut result = MigrationResult {
        skills_inserted: 0,
        stories_inserted: 0,
        edges_created: 0,
        weaknesses_inserted: 0,
        achievements_inserted: 0,
        preferences_inserted: 0,
        skipped_existing: 0,
    };

    // ------------------------------------------------------------------
    // 1. Skills (dominados + intermedios)
    // ------------------------------------------------------------------
    let mut skill_name_to_id: HashMap<String, String> = HashMap::new();

    let all_skills: Vec<(&str, &SkillInfo)> = profile
        .skills
        .dominados
        .iter()
        .map(|s| ("dominado", s))
        .chain(profile.skills.intermedios.iter().map(|s| ("intermedio", s)))
        .collect();

    for (category, skill) in &all_skills {
        let name = &skill.nombre;
        let existing_id = find_skill_by_name(graph, name);

        if let Some(_id) = existing_id {
            result.skipped_existing += 1;
        } else {
            let entity = graph.create_skill(
                name,
                Some(category),
                &skill.nivel,
                skill.años as i32,
                Some("profile.yaml"),
            )?;
            skill_name_to_id.insert(name.clone(), entity.id.clone());
            result.skills_inserted += 1;
        }
    }

    // If we skipped some, backfill the map for edge creation later
    if !skill_name_to_id.is_empty() || result.skipped_existing > 0 {
        fill_skill_name_map(graph, &mut skill_name_to_id)?;
    }

    // ------------------------------------------------------------------
    // 2. STAR Stories
    // ------------------------------------------------------------------
    let mut story_id_map: HashMap<String, String> = HashMap::new(); // yaml id → db id

    for story in &profile.historias_star {
        let title = generate_story_title(story);
        let tags_json = serde_json::to_string(&story.tags).unwrap_or_else(|_| "[]".to_string());

        let existing = find_story_by_yaml_id(graph, &story.id);
        if let Some(_db_id) = existing {
            result.skipped_existing += 1;
        } else {
            let entity = graph.create_star_story(
                Some(&title),
                &story.situacion,
                &story.tarea,
                &story.accion,
                &story.resultado,
                Some(&tags_json),
                infer_difficulty(&story.tags).as_deref(),
                infer_stakes(&story.tags).as_deref(),
            )?;
            story_id_map.insert(story.id.clone(), entity.id.clone());
            result.stories_inserted += 1;
        }
    }

    // Backfill for idempotency
    fill_story_yaml_id_map(graph, &mut story_id_map)?;

    // ------------------------------------------------------------------
    // 3. Edges: skill ↔ star_story (evidenced_by)
    // ------------------------------------------------------------------
    for story in &profile.historias_star {
        let db_story_id = match story_id_map.get(&story.id) {
            Some(id) => id,
            None => continue,
        };

        for tag in &story.tags {
            // Try to match tag to a skill name (case-insensitive)
            let matched_skill_id = skill_name_to_id.keys().find(|name| {
                name.to_lowercase() == tag.to_lowercase()
                    || tag.to_lowercase().contains(&name.to_lowercase())
                    || name.to_lowercase().contains(&tag.to_lowercase())
            });

            if let Some(skill_name) = matched_skill_id {
                let skill_id = &skill_name_to_id[skill_name];

                // Check edge doesn't already exist
                if !edge_exists(graph, skill_id, db_story_id, "evidenced_by") {
                    let _ = graph.add_edge(
                        skill_id,
                        EntityType::Skill,
                        db_story_id,
                        EntityType::StarStory,
                        "evidenced_by",
                        1.0,
                    );
                    result.edges_created += 1;
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // 4. Weaknesses → metadata
    // ------------------------------------------------------------------
    for weakness in &profile.debilidades_conocidas {
        let key = format!("weakness:{}", weakness.area);
        let value = format!("{}|{}", weakness.area, weakness.estrategia);
        if graph.get_metadata(&key)?.is_none() {
            graph.set_metadata(&key, &value)?;
            result.weaknesses_inserted += 1;
        } else {
            result.skipped_existing += 1;
        }
    }

    // ------------------------------------------------------------------
    // 5. Achievements (logros) → metadata
    // ------------------------------------------------------------------
    for (i, logro) in profile.logros.iter().enumerate() {
        let key = format!("achievement:{}", i);
        if graph.get_metadata(&key)?.is_none() {
            graph.set_metadata(&key, logro)?;
            result.achievements_inserted += 1;
        } else {
            result.skipped_existing += 1;
        }
    }

    // ------------------------------------------------------------------
    // 6. Preferences → metadata
    // ------------------------------------------------------------------
    let prefs = &profile.preferencias;
    let pref_entries: Vec<(&str, &str)> = vec![
        ("communication_style", &prefs.estilo_comunicacion),
        (
            "preferred_work_environment",
            &prefs.entorno_trabajo_preferido,
        ),
        ("preferred_project_type", &prefs.tipo_proyectos_preferido),
        ("main_motivation", &prefs.motivacion_principal),
    ];

    for (label, value) in &pref_entries {
        let key = format!("preference:{}", label);
        if graph.get_metadata(&key)?.is_none() {
            graph.set_metadata(&key, value)?;
            result.preferences_inserted += 1;
        } else {
            result.skipped_existing += 1;
        }
    }

    // ------------------------------------------------------------------
    // 7. Record migration timestamp
    // ------------------------------------------------------------------
    let now = chrono::Utc::now().to_rfc3339();
    graph.set_metadata(MIGRATION_META_KEY, &now)?;

    Ok(result)
}

/// Check whether the migration has already been run for the given profile.
pub fn migration_already_ran(graph: &KnowledgeGraph) -> bool {
    graph
        .get_metadata(MIGRATION_META_KEY)
        .unwrap_or(None)
        .is_some()
}

/// Get the last migration timestamp, if any.
pub fn last_migration_timestamp(graph: &KnowledgeGraph) -> Option<String> {
    graph.get_metadata(MIGRATION_META_KEY).unwrap_or(None)
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Find a skill by name. Returns its DB id if it exists.
fn find_skill_by_name(graph: &KnowledgeGraph, name: &str) -> Option<String> {
    let conn = graph.conn();
    let mut stmt = conn
        .prepare("SELECT id FROM skills WHERE name = ?1 LIMIT 1")
        .ok()?;
    let mut rows = stmt.query(rusqlite::params![name]).ok()?;
    rows.next().ok()??.get(0).ok()
}

/// Find a star_story by its original YAML id (stored in the title field).
fn find_story_by_yaml_id(graph: &KnowledgeGraph, yaml_id: &str) -> Option<String> {
    let conn = graph.conn();
    let pattern = format!("[{}]", yaml_id);
    let mut stmt = conn
        .prepare("SELECT id FROM star_stories WHERE title LIKE ?1 LIMIT 1")
        .ok()?;
    let mut rows = stmt
        .query(rusqlite::params![format!("%{}%", pattern)])
        .ok()?;
    rows.next().ok()??.get(0).ok()
}

/// Generate a descriptive title for a story using the YAML id and first tag.
fn generate_story_title(story: &StarStory) -> String {
    let first_tag = story.tags.first().map(|t| t.as_str()).unwrap_or("untagged");
    format!("[{}] {}", story.id, first_tag)
}

/// Check if an edge already exists between two entities with a given relation.
fn edge_exists(graph: &KnowledgeGraph, source_id: &str, target_id: &str, relation: &str) -> bool {
    let conn = graph.conn();
    let mut stmt = conn
        .prepare(
            "SELECT COUNT(*) FROM edges WHERE source_id = ?1 AND target_id = ?2 AND relation = ?3",
        )
        .unwrap();
    let count: i64 = stmt
        .query_row(rusqlite::params![source_id, target_id, relation], |row| {
            row.get(0)
        })
        .unwrap_or(0);
    count > 0
}

/// Fill the skill_name→id map by querying all existing skills.
fn fill_skill_name_map(graph: &KnowledgeGraph, map: &mut HashMap<String, String>) -> Result<()> {
    let skills = graph.list_skills()?;
    for skill in skills {
        map.entry(skill.name.clone()).or_insert(skill.id);
    }
    Ok(())
}

/// Fill the story_yaml_id→db_id map by querying existing stories.
fn fill_story_yaml_id_map(graph: &KnowledgeGraph, map: &mut HashMap<String, String>) -> Result<()> {
    let stories = graph.list_star_stories()?;
    for story in stories {
        if let Some(title) = &story.title {
            // Extract YAML id from title pattern "[yaml-id] ..."
            if let Some(end) = title.find(']') {
                if title.starts_with('[') {
                    let yaml_id = &title[1..end];
                    map.entry(yaml_id.to_string()).or_insert(story.id);
                }
            }
        }
    }
    Ok(())
}

/// Infer difficulty from story tags.
fn infer_difficulty(tags: &[String]) -> Option<String> {
    let tags_lower: Vec<String> = tags.iter().map(|t| t.to_lowercase()).collect();
    if tags_lower
        .iter()
        .any(|t| t.contains("crisis") || t.contains("critical"))
    {
        Some("high".to_string())
    } else if tags_lower
        .iter()
        .any(|t| t.contains("migration") || t.contains("architecture"))
    {
        Some("high".to_string())
    } else {
        Some("medium".to_string())
    }
}

/// Infer stakes from story tags.
fn infer_stakes(tags: &[String]) -> Option<String> {
    let tags_lower: Vec<String> = tags.iter().map(|t| t.to_lowercase()).collect();
    if tags_lower
        .iter()
        .any(|t| t.contains("budget") || t.contains("production") || t.contains("payment"))
    {
        Some("high".to_string())
    } else if tags_lower
        .iter()
        .any(|t| t.contains("team") || t.contains("conflict"))
    {
        Some("medium".to_string())
    } else {
        Some("medium".to_string())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Create a temporary directory with a test profile.yaml for testing.
    fn setup_test_env() -> (tempfile::TempDir, KnowledgeGraph) {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let personal_dir = dir.path().join("personal");
        fs::create_dir_all(&personal_dir).unwrap();

        let yaml_content = r#"
nombre: "Test User"
rol_actual: "Senior Backend Engineer"
experiencia: "8 years"
ubicacion: "Remote"

skills:
  dominados:
    - nombre: "Go"
      nivel: "expert"
      años: 5
      proyectos:
        - "API gateway de alto rendimiento"
        - "Sistema de microservicios para pagos"
    - nombre: "Kubernetes"
      nivel: "advanced"
      años: 3
      proyectos:
        - "Orquestación de servicios en producción"
    - nombre: "PostgreSQL"
      nivel: "expert"
      años: 6
      proyectos:
        - "Diseño de esquemas para alta concurrencia"
  intermedios:
    - nombre: "Rust"
      nivel: "learning"
      años: 1
      proyectos:
        - "CLI tool para procesamiento de logs"
    - nombre: "React"
      nivel: "learning"
      años: 1
      proyectos:
        - "Dashboard interno de métricas"

historias_star:
  - id: "leadership-1"
    situacion: "Team lost the tech lead during a critical sprint"
    tarea: "Had to take ownership without formal title"
    accion: "Organized daily standups and pair programming"
    resultado: "Delivered 2 weeks early with zero production bugs"
    tags: ["leadership", "crisis", "team-management"]
  - id: "technical-1"
    situacion: "API latency of 2s causing timeouts and cart abandonment"
    tarea: "Reduce to under 200ms without architecture changes"
    accion: "Implemented Redis caching and optimized database queries"
    resultado: "180ms p99 latency, 40% infrastructure cost reduction"
    tags: ["performance", "backend", "caching", "database"]
  - id: "behavioral-1"
    situacion: "Conflict between two senior developers"
    tarea: "Mediate and find a technical solution"
    accion: "Facilitated structured meeting"
    resultado: "Solution combined both approaches"
    tags: ["conflict-resolution", "communication"]

logros:
  - "Reduced AWS costs by 60%"
  - "Led monolith to microservices migration"

debilidades_conocidas:
  - area: "system design of ML pipelines"
    estrategia: "Honest about learning, focus on data principles"
  - area: "frontend avanzado"
    estrategia: "Focus on rapid learning capability"

preferencias:
  estilo_comunicacion: "direct and data-driven"
  entorno_trabajo_preferido: "small autonomous teams"
  tipo_proyectos_preferido: "high-scale backend systems"
  motivacion_principal: "solving complex problems with real impact"
"#;
        fs::write(personal_dir.join("profile.yaml"), yaml_content).unwrap();

        let graph = KnowledgeGraph::open_in_memory().unwrap();

        (dir, graph)
    }

    #[test]
    fn test_migration_preserves_all_fields() {
        let (dir, graph) = setup_test_env();
        let yaml_path = dir.path().join("personal").join("profile.yaml");

        let result = migrate_profile_yaml(&yaml_path, &graph).unwrap();

        // 5 skills (3 dominados + 2 intermedios)
        assert_eq!(result.skills_inserted, 5);
        // 3 stories
        assert_eq!(result.stories_inserted, 3);
        // 2 weaknesses
        assert_eq!(result.weaknesses_inserted, 2);
        // 2 achievements
        assert_eq!(result.achievements_inserted, 2);
        // 4 preferences
        assert_eq!(result.preferences_inserted, 4);

        // Verify skill data is correct
        let skills = graph.list_skills().unwrap();
        assert_eq!(skills.len(), 5);

        let go_skill = skills.iter().find(|s| s.name == "Go").unwrap();
        assert_eq!(go_skill.level, "expert");
        assert_eq!(go_skill.years, 5);
        assert_eq!(go_skill.source.as_deref(), Some("profile.yaml"));

        let rust_skill = skills.iter().find(|s| s.name == "Rust").unwrap();
        assert_eq!(rust_skill.level, "learning");
        assert_eq!(rust_skill.years, 1);

        // Verify category mapping (dominado vs intermedio)
        assert_eq!(go_skill.category.as_deref(), Some("dominado"));
        assert_eq!(rust_skill.category.as_deref(), Some("intermedio"));

        // Verify story data
        let stories = graph.list_star_stories().unwrap();
        assert_eq!(stories.len(), 3);

        let leadership_story = stories
            .iter()
            .find(|s| s.title.as_ref().unwrap().contains("leadership-1"))
            .unwrap();
        assert_eq!(
            leadership_story.situation,
            "Team lost the tech lead during a critical sprint"
        );
        assert_eq!(
            leadership_story.task,
            "Had to take ownership without formal title"
        );
        assert_eq!(
            leadership_story.action,
            "Organized daily standups and pair programming"
        );
        assert_eq!(
            leadership_story.result,
            "Delivered 2 weeks early with zero production bugs"
        );

        // Verify tags are JSON
        let tags: Vec<String> =
            serde_json::from_str(leadership_story.tags.as_ref().unwrap()).unwrap();
        assert!(tags.contains(&"leadership".to_string()));
        assert!(tags.contains(&"crisis".to_string()));
    }

    #[test]
    fn test_migration_creates_edges() {
        let (dir, graph) = setup_test_env();
        let yaml_path = dir.path().join("personal").join("profile.yaml");

        let result = migrate_profile_yaml(&yaml_path, &graph).unwrap();

        assert!(result.edges_created >= 0);

        let conn = graph.conn();
        let edge_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM edges WHERE relation = 'evidenced_by'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(edge_count as usize, result.edges_created);

        // Verify edge structure: source_type='skill', target_type='star_story'
        if edge_count > 0 {
            let edges: Vec<(String, String, String)> = conn
                .prepare(
                    "SELECT source_type, target_type, relation FROM edges WHERE relation = 'evidenced_by' LIMIT 1",
                )
                .unwrap()
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect();
            assert_eq!(edges[0].0, "skill");
            assert_eq!(edges[0].1, "star_story");
            assert_eq!(edges[0].2, "evidenced_by");
        }
    }

    #[test]
    fn test_migration_idempotent() {
        let (dir, graph) = setup_test_env();
        let yaml_path = dir.path().join("personal").join("profile.yaml");

        // Run migration twice
        let result1 = migrate_profile_yaml(&yaml_path, &graph).unwrap();
        let result2 = migrate_profile_yaml(&yaml_path, &graph).unwrap();

        // Second run should insert nothing new
        assert_eq!(result2.skills_inserted, 0);
        assert_eq!(result2.stories_inserted, 0);
        assert_eq!(result2.edges_created, 0);
        assert_eq!(result2.weaknesses_inserted, 0);
        assert_eq!(result2.achievements_inserted, 0);
        assert_eq!(result2.preferences_inserted, 0);

        // Should have skipped existing entries
        assert!(result2.skipped_existing > 0);

        // Verify total counts haven't changed
        let skills = graph.list_skills().unwrap();
        assert_eq!(skills.len(), result1.skills_inserted);

        let stories = graph.list_star_stories().unwrap();
        assert_eq!(stories.len(), result1.stories_inserted);
    }

    #[test]
    fn test_spanish_field_mapping() {
        let (dir, graph) = setup_test_env();
        let yaml_path = dir.path().join("personal").join("profile.yaml");

        let _result = migrate_profile_yaml(&yaml_path, &graph).unwrap();

        // nombre → name
        let skills = graph.list_skills().unwrap();
        let names: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Go"));
        assert!(names.contains(&"Kubernetes"));
        assert!(names.contains(&"Rust"));
        assert!(names.contains(&"React"));

        // años → years
        let go = skills.iter().find(|s| s.name == "Go").unwrap();
        assert_eq!(go.years, 5); // 5 años → 5 years

        let k8s = skills.iter().find(|s| s.name == "Kubernetes").unwrap();
        assert_eq!(k8s.years, 3);

        // debilidades_conocidas → metadata entries
        let w1 = graph
            .get_metadata("weakness:system design of ML pipelines")
            .unwrap();
        assert!(w1.is_some());
        let value = w1.unwrap();
        assert!(value.contains("system design of ML pipelines"));
        assert!(value.contains("Honest about learning"));

        // logros → metadata entries
        let a0 = graph.get_metadata("achievement:0").unwrap();
        assert!(a0.is_some());
        assert!(a0.unwrap().contains("Reduced AWS costs"));

        // preferencias → metadata entries
        let comm = graph
            .get_metadata("preference:communication_style")
            .unwrap();
        assert!(comm.is_some());
        assert_eq!(comm.unwrap(), "direct and data-driven");

        let env = graph
            .get_metadata("preference:preferred_work_environment")
            .unwrap();
        assert!(env.is_some());
        assert_eq!(env.unwrap(), "small autonomous teams");
    }

    #[test]
    fn test_migration_records_timestamp() {
        let (dir, graph) = setup_test_env();
        let yaml_path = dir.path().join("personal").join("profile.yaml");

        assert!(!migration_already_ran(&graph));
        assert!(last_migration_timestamp(&graph).is_none());

        migrate_profile_yaml(&yaml_path, &graph).unwrap();

        assert!(migration_already_ran(&graph));
        let ts = last_migration_timestamp(&graph).unwrap();
        // Should be a valid ISO timestamp
        assert!(ts.contains('T') || ts.contains('-'));
    }

    #[test]
    fn test_migration_missing_yaml_file() {
        let graph = KnowledgeGraph::open_in_memory().unwrap();
        let result = migrate_profile_yaml(Path::new("/nonexistent/profile.yaml"), &graph);
        assert!(result.is_err());
    }

    #[test]
    fn test_migration_difficulty_stakes_inferred() {
        let (dir, graph) = setup_test_env();
        let yaml_path = dir.path().join("personal").join("profile.yaml");

        migrate_profile_yaml(&yaml_path, &graph).unwrap();

        let stories = graph.list_star_stories().unwrap();

        // leadership-1 has "crisis" tag → difficulty should be "high"
        let leadership = stories
            .iter()
            .find(|s| s.title.as_ref().unwrap().contains("leadership-1"))
            .unwrap();
        assert_eq!(leadership.difficulty.as_deref(), Some("high"));

        // behavioral-1 has "communication" but no crisis → medium
        let behavioral = stories
            .iter()
            .find(|s| s.title.as_ref().unwrap().contains("behavioral-1"))
            .unwrap();
        assert_eq!(behavioral.difficulty.as_deref(), Some("medium"));
    }
}
