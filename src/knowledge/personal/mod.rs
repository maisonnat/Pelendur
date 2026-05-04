use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Representa el perfil personal del usuario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalProfile {
    pub nombre: String,
    pub rol_actual: String,
    pub experiencia: String,
    pub ubicacion: Option<String>,
    pub email: Option<String>,
    pub linkedin: Option<String>,

    pub idiomas: Vec<Idioma>,
    pub educacion: Vec<Educacion>,
    pub certificaciones: Vec<String>,

    pub skills: Skills,
    pub historias_star: Vec<StarStory>,
    pub logros: Vec<String>,
    pub debilidades_conocidas: Vec<DebilidadConocida>,
    pub preferencias: Preferencias,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Idioma {
    pub idioma: String,
    pub nivel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Educacion {
    pub titulo: String,
    pub institucion: String,
    #[serde(default)]
    pub estado: Option<String>,
    #[serde(default)]
    pub periodo: Option<String>,
    #[serde(default)]
    pub expected: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skills {
    pub dominados: Vec<SkillInfo>,
    pub intermedios: Vec<SkillInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    pub nombre: String,
    pub nivel: String,
    pub años: u8,
    pub proyectos: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarStory {
    pub id: String,
    pub situacion: String,
    pub tarea: String,
    pub accion: String,
    pub resultado: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebilidadConocida {
    pub area: String,
    pub estrategia: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferencias {
    pub estilo_comunicacion: String,
    pub entorno_trabajo_preferido: String,
    pub tipo_proyectos_preferido: String,
    pub motivacion_principal: String,
}

impl PersonalProfile {
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let profile: PersonalProfile = serde_yaml::from_str(&content)?;
        Ok(profile)
    }

    pub fn find_relevant_stories(&self, query: &str) -> Vec<&StarStory> {
        let query_lower = query.to_lowercase();
        self.historias_star
            .iter()
            .filter(|story| {
                story
                    .tags
                    .iter()
                    .any(|tag| query_lower.contains(&tag.to_lowercase()))
                    || story.situacion.to_lowercase().contains(&query_lower)
                    || story.tarea.to_lowercase().contains(&query_lower)
            })
            .collect()
    }
}

pub trait KnowledgeProvider: Send + Sync {
    fn get_name(&self) -> &str;
    fn search(&self, query: &str) -> Result<Vec<String>>;
    fn save_observation(&self, title: &str, content: &str) -> Result<()>;
}

pub struct FileKnowledgeProvider {
    pub base_path: String,
}

impl KnowledgeProvider for FileKnowledgeProvider {
    fn get_name(&self) -> &str {
        "LocalFiles"
    }
    fn search(&self, query: &str) -> Result<Vec<String>> {
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();

        // 1. Buscar en skills
        let skills_dir = format!("{}/skills", self.base_path);
        if let Ok(entries) = fs::read_dir(skills_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    let overview_path = entry.path().join("overview.md");
                    if let Ok(content) = fs::read_to_string(overview_path) {
                        if content.to_lowercase().contains(&query_lower) {
                            results.push(format!(
                                "Skill [{}]: {}",
                                entry.file_name().to_string_lossy(),
                                content.lines().next().unwrap_or("")
                            ));
                        }
                    }
                }
            }
        }

        // 2. Buscar en companies (Capa 3)
        let companies_dir = format!("{}/companies", self.base_path);
        if let Ok(entries) = fs::read_dir(companies_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    let overview_path = entry.path().join("overview.md");
                    if let Ok(content) = fs::read_to_string(overview_path) {
                        if content.to_lowercase().contains(&query_lower)
                            || query_lower
                                .contains(&entry.file_name().to_string_lossy().to_lowercase())
                        {
                            results.push(format!(
                                "Company Research [{}]: {}",
                                entry.file_name().to_string_lossy(),
                                content.lines().take(5).collect::<Vec<_>>().join(" ")
                            ));
                        }
                    }
                }
            }
        }

        Ok(results)
    }
    fn save_observation(&self, title: &str, content: &str) -> Result<()> {
        let path = format!(
            "{}/interviews/{}.md",
            self.base_path,
            title.replace(" ", "_").replace(":", "")
        );
        fs::create_dir_all(format!("{}/interviews", self.base_path))?;
        fs::write(path, content)?;
        Ok(())
    }
}

pub struct EngramKnowledgeProvider {
    pub binary_path: String,
    pub project: String,
}

impl KnowledgeProvider for EngramKnowledgeProvider {
    fn get_name(&self) -> &str {
        "CrabEngram"
    }
    fn search(&self, query: &str) -> Result<Vec<String>> {
        use std::process::Command;
        let output = Command::new(&self.binary_path)
            .arg("--project")
            .arg(&self.project)
            .arg("search")
            .arg(query)
            .output()?;

        if output.status.success() {
            let result_str = String::from_utf8_lossy(&output.stdout);
            Ok(result_str.lines().take(3).map(|s| s.to_string()).collect())
        } else {
            Err(anyhow::anyhow!("Engram search failed"))
        }
    }
    fn save_observation(&self, title: &str, content: &str) -> Result<()> {
        use std::process::Command;
        // The Crab Engram CLI requires explicit flags for save:
        // save --title <TITLE> --content <CONTENT> --session-id <SESSION_ID>
        let status = Command::new(&self.binary_path)
            .arg("--project")
            .arg(&self.project)
            .arg("save")
            .arg("--title")
            .arg(title)
            .arg("--content")
            .arg(content)
            .arg("--session-id")
            .arg("pelendur-cli-session")
            .status()?;

        if status.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Engram save failed"))
        }
    }
}

pub struct KnowledgeManager {
    pub personal_profile: Option<PersonalProfile>,
    pub providers: Vec<Box<dyn KnowledgeProvider>>,
    pub knowledge_base_path: String,
    pub graph_provider: Option<crate::knowledge::graph::GraphKnowledgeProvider>,
}

impl KnowledgeManager {
    pub fn new(knowledge_base_path: &str) -> Self {
        let mut providers: Vec<Box<dyn KnowledgeProvider>> = Vec::new();
        providers.push(Box::new(FileKnowledgeProvider {
            base_path: knowledge_base_path.to_string(),
        }));

        let engram_path = "C:\\Proyectos\\engram-rust\\target\\release\\the-crab-engram.exe";
        if Path::new(engram_path).exists() {
            providers.push(Box::new(EngramKnowledgeProvider {
                binary_path: engram_path.to_string(),
                project: "pelendur".to_string(),
            }));
        }

        Self {
            personal_profile: None,
            providers,
            knowledge_base_path: knowledge_base_path.to_string(),
            graph_provider: None,
        }
    }

    pub fn with_graph_provider(
        mut self,
        gp: crate::knowledge::graph::GraphKnowledgeProvider,
    ) -> Self {
        self.graph_provider = Some(gp);
        self
    }

    pub fn set_graph_provider(&mut self, gp: crate::knowledge::graph::GraphKnowledgeProvider) {
        self.graph_provider = Some(gp);
    }

    pub fn load_personal_profile(&mut self) -> Result<()> {
        let profile_path = format!("{}/personal/profile.yaml", self.knowledge_base_path);
        let profile = PersonalProfile::load_from_file(Path::new(&profile_path))?;
        self.personal_profile = Some(profile);
        Ok(())
    }

    pub fn search_all(&self, query: &str) -> Vec<String> {
        let mut all_results = Vec::new();

        for provider in &self.providers {
            if let Ok(results) = provider.search(query) {
                for res in results {
                    all_results.push(format!("[{}] {}", provider.get_name(), res));
                }
            }
        }

        if let Some(ref gp) = self.graph_provider {
            let graph = gp.graph();
            let searcher = crate::knowledge::search::KnowledgeSearcher::new(&graph);
            if let Ok(results) = searcher.search_all_entities(query) {
                for r in results {
                    all_results.push(format!(
                        "[{}] {} — {} (score: {:.2})",
                        r.entity_type, r.name, r.snippet, r.relevance_score
                    ));
                }
            }
        }

        all_results
    }

    pub fn find_relevant_stories_enhanced(
        &self,
        query: &str,
    ) -> Vec<crate::knowledge::search::SearchResult> {
        let mut all_results = Vec::new();

        if let Some(ref gp) = self.graph_provider {
            let graph = gp.graph();
            let searcher = crate::knowledge::search::KnowledgeSearcher::new(&graph);
            all_results.extend(searcher.context_search(query));
        }

        if let Some(ref profile) = self.personal_profile {
            for story in &profile.historias_star {
                let query_lower = query.to_lowercase();
                let story_text = format!(
                    "{} {} {} {} {}",
                    story.situacion,
                    story.tarea,
                    story.accion,
                    story.resultado,
                    story.tags.join(" ")
                );
                if story_text.to_lowercase().contains(&query_lower) {
                    all_results.push(crate::knowledge::search::SearchResult {
                        entity_type: "star_story".to_string(),
                        entity_id: story.id.clone(),
                        name: story.id.clone(),
                        relevance_score: 1.0,
                        matched_terms: vec![query.to_string()],
                        snippet: format!("{} -> {}", story.situacion, story.resultado),
                        data: Some(serde_json::json!({
                            "situation": story.situacion,
                            "task": story.tarea,
                            "action": story.accion,
                            "result": story.resultado,
                            "tags": story.tags,
                        })),
                    });
                }
            }
        }

        let mut seen: std::collections::HashMap<String, crate::knowledge::search::SearchResult> =
            std::collections::HashMap::new();
        for r in all_results {
            let r_score = r.relevance_score;
            seen.entry(r.entity_id.clone())
                .and_modify(|existing| {
                    if r_score > existing.relevance_score {
                        *existing = r.clone();
                    }
                })
                .or_insert(r);
        }

        let mut results: Vec<_> = seen.into_values().collect();
        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results
    }

    pub fn save_to_all(&self, title: &str, content: &str) {
        for provider in &self.providers {
            if let Err(e) = provider.save_observation(title, content) {
                eprintln!("Failed to save to provider {}: {}", provider.get_name(), e);
            }
        }
    }

    pub fn semantic_search(
        &self,
        query: &str,
        top_k: usize,
    ) -> Vec<crate::knowledge::search::SearchResult> {
        let mut all_results = Vec::new();

        if let Some(ref gp) = self.graph_provider {
            let graph = gp.graph();
            let searcher = crate::knowledge::search::KnowledgeSearcher::new(&graph);
            if let Ok(results) = searcher.semantic_search(query, top_k) {
                all_results.extend(results);
            }
        }

        all_results
    }

    pub fn hybrid_search(
        &self,
        query: &str,
        top_k: usize,
    ) -> Vec<crate::knowledge::search::SearchResult> {
        let mut all_results = Vec::new();

        if let Some(ref gp) = self.graph_provider {
            let graph = gp.graph();
            let searcher = crate::knowledge::search::KnowledgeSearcher::new(&graph);
            if let Ok(results) = searcher.hybrid_search(
                query,
                crate::knowledge::search::KnowledgeSearchOptions {
                    max_results: top_k,
                    fuzzy_threshold: 0.6,
                    include_types: Vec::new(),
                    tag_filter: None,
                },
            ) {
                all_results.extend(results);
            }
        }

        all_results
    }

    pub fn generate_all_embeddings(&self) -> Result<()> {
        if let Some(ref gp) = self.graph_provider {
            let graph = gp.graph();
            graph.generate_embeddings()?;
        }
        Ok(())
    }
}

/// Genera el prompt del sistema dinámicamente basado en el perfil del usuario
pub fn generate_system_prompt(manager: &KnowledgeManager) -> String {
    let mut prompt = String::from(
        r#"You are Pelendur, an advanced AI Interview Copilot that knows the candidate deeply.
You receive transcribed audio from a conversation and provide concise, high-value assistance.

IMPORTANT: Always respond in the SAME LANGUAGE as the transcribed text you receive.
Be extremely brief — 2-3 sentences max. Focus on providing unfair advantages.

Your goal is to suggest the best STAR stories, technical concepts, or strategic advice 
based on the candidate's actual experience and the company's context.
"#,
    );

    if let Some(profile) = &manager.personal_profile {
        prompt.push_str("\n### CANDIDATE PROFILE:\n");
        prompt.push_str(&format!("Name: {}\n", profile.nombre));
        prompt.push_str(&format!("Current Role: {}\n", profile.rol_actual));
        prompt.push_str(&format!("Experience: {}\n", profile.experiencia));
        if let Some(email) = &profile.email {
            prompt.push_str(&format!("Email: {}\n", email));
        }
        if let Some(linkedin) = &profile.linkedin {
            prompt.push_str(&format!("LinkedIn: {}\n", linkedin));
        }

        if !profile.idiomas.is_empty() {
            prompt.push_str("\n### LANGUAGES:\n");
            for lang in &profile.idiomas {
                prompt.push_str(&format!("- {} ({})\n", lang.idioma, lang.nivel));
            }
        }

        if !profile.educacion.is_empty() {
            prompt.push_str("\n### EDUCATION:\n");
            for edu in &profile.educacion {
                if let Some(period) = &edu.periodo {
                    prompt.push_str(&format!("- {} — {} ({})\n", edu.titulo, edu.institucion, period));
                } else {
                    prompt.push_str(&format!("- {} — {}\n", edu.titulo, edu.institucion));
                }
            }
        }

        if !profile.certificaciones.is_empty() {
            prompt.push_str(&format!("\n### CERTIFICATIONS:\n- {}\n", profile.certificaciones.join("\n- ")));
        }

        prompt.push_str("\n### TOP SKILLS:\n");
        for skill in profile.skills.dominados.iter().take(5) {
            prompt.push_str(&format!("- {}: {} years\n", skill.nombre, skill.años));
        }
    }

    prompt.push_str(
        r#"
When you see a matching opportunity:
1. "Use STAR [ID]: [Short summary of the story]"
2. "Technical tip: [Brief concept or metric from your projects]"
3. "Caution: [Strategy for a weakness or common trap]"
"#,
    );

    prompt
}

/// Generates system prompt with company research context for interview mode
pub fn generate_company_interview_prompt(
    manager: &KnowledgeManager,
    company_name: &str,
) -> String {
    let mut prompt = generate_system_prompt(manager);

    // Inject company research context
    if let Ok(context) = crate::knowledge::company::CompanyLoader::new(&manager.knowledge_base_path)
        .get_interview_context(company_name)
    {
        prompt.push_str("\n\n### COMPANY RESEARCH CONTEXT:\n");
        prompt.push_str(&context);
        prompt.push_str("\n\nUse this company context to tailor your suggestions.\n");
    }

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Helper to create a temporary directory with a test profile
    fn setup_test_kb() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let kb_path = dir.path();

        // Create personal/profile.yaml
        let personal_dir = kb_path.join("personal");
        fs::create_dir_all(&personal_dir).unwrap();
        let profile_yaml = r#"
nombre: "Test User"
rol_actual: "Senior Backend Engineer"
experiencia: "8 years"
ubicacion: "Remote"
email: "test@example.com"
linkedin: "linkedin.com/in/testuser"

idiomas:
  - idioma: "English"
    nivel: "Native"
  - idioma: "Spanish"
    nivel: "Professional"

educacion:
  - titulo: "BS Computer Science"
    institucion: "Tech University"
    periodo: "2010-2014"
  - titulo: "MSc Data Science"
    institucion: "Data Institute"
    estado: "In progress"
    expected: "December 2026"

certificaciones:
  - "AWS Certified Solutions Architect"
  - "Certified Kubernetes Administrator"

skills:
  dominados:
    - nombre: "Go"
      nivel: "expert"
      años: 5
      proyectos: ["API gateway"]
    - nombre: "Kubernetes"
      nivel: "advanced"
      años: 3
      proyectos: ["Service orchestration"]
  intermedios:
    - nombre: "Rust"
      nivel: "learning"
      años: 1
      proyectos: ["CLI tool"]

historias_star:
  - id: "test-leadership-1"
    situacion: "Team lost the tech lead during a critical sprint"
    tarea: "Had to take ownership without formal title"
    accion: "Organized daily standups and pair programming"
    resultado: "Delivered 2 weeks early with zero production bugs"
    tags: ["leadership", "crisis", "team-management"]
  - id: "test-perf-1"
    situacion: "API latency of 2s causing timeouts and cart abandonment"
    tarea: "Reduce to under 200ms without architecture changes"
    accion: "Implemented Redis caching and optimized database queries"
    resultado: "180ms p99 latency, 40% infrastructure cost reduction"
    tags: ["performance", "backend", "caching", "database"]

logros:
  - "Reduced AWS costs by 60%"
  - "Led monolith to microservices migration"

debilidades_conocidas:
  - area: "system design of ML pipelines"
    estrategia: "Honest about learning, focus on data principles I know"

preferencias:
  estilo_comunicacion: "direct and data-driven"
  entorno_trabajo_preferido: "small autonomous teams"
  tipo_proyectos_preferido: "high-scale backend systems"
  motivacion_principal: "solving complex problems with real impact"
"#;
        fs::write(personal_dir.join("profile.yaml"), profile_yaml).unwrap();

        // Create skills/skill-a/overview.md
        let skills_dir = kb_path.join("skills").join("skill-a");
        fs::create_dir_all(&skills_dir).unwrap();
        fs::write(skills_dir.join("overview.md"), "# Skill A\nGo programming language expertise with focus on concurrency and microservices.").unwrap();

        // Create companies/acme/overview.md
        let companies_dir = kb_path.join("companies").join("acme");
        fs::create_dir_all(&companies_dir).unwrap();
        fs::write(companies_dir.join("overview.md"), "# Acme Corp\nCloud infrastructure company. Stack: Kubernetes, Go, PostgreSQL. Engineering-driven culture.").unwrap();

        // Create interviews dir
        fs::create_dir_all(kb_path.join("interviews")).unwrap();

        dir
    }

    // ========================
    // PersonalProfile Tests
    // ========================

    #[test]
    fn test_load_profile_from_file() {
        let dir = setup_test_kb();
        let profile_path = dir.path().join("personal").join("profile.yaml");
        let profile = PersonalProfile::load_from_file(&profile_path).unwrap();

        assert_eq!(profile.nombre, "Test User");
        assert_eq!(profile.rol_actual, "Senior Backend Engineer");
        assert_eq!(profile.skills.dominados.len(), 2);
        assert_eq!(profile.skills.intermedios.len(), 1);
        assert_eq!(profile.historias_star.len(), 2);
    }

    #[test]
    fn test_load_profile_missing_file() {
        let result = PersonalProfile::load_from_file(Path::new("/nonexistent/profile.yaml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_find_relevant_stories_by_tag() {
        let dir = setup_test_kb();
        let profile =
            PersonalProfile::load_from_file(&dir.path().join("personal").join("profile.yaml"))
                .unwrap();

        let stories = profile.find_relevant_stories("tell me about a leadership challenge");
        assert_eq!(stories.len(), 1);
        assert_eq!(stories[0].id, "test-leadership-1");
    }

    #[test]
    fn test_find_relevant_stories_by_situation() {
        let dir = setup_test_kb();
        let profile =
            PersonalProfile::load_from_file(&dir.path().join("personal").join("profile.yaml"))
                .unwrap();

        let stories = profile.find_relevant_stories("latency of 2s causing timeouts");
        assert_eq!(stories.len(), 1);
        assert_eq!(stories[0].id, "test-perf-1");
    }

    #[test]
    fn test_find_relevant_stories_no_match() {
        let dir = setup_test_kb();
        let profile =
            PersonalProfile::load_from_file(&dir.path().join("personal").join("profile.yaml"))
                .unwrap();

        let stories =
            profile.find_relevant_stories("tell me about your experience with React animations");
        assert!(stories.is_empty());
    }

    // ========================
    // FileKnowledgeProvider Tests
    // ========================

    #[test]
    fn test_file_provider_name() {
        let provider = FileKnowledgeProvider {
            base_path: "/tmp".to_string(),
        };
        assert_eq!(provider.get_name(), "LocalFiles");
    }

    #[test]
    fn test_file_provider_search_skills() {
        let dir = setup_test_kb();
        let provider = FileKnowledgeProvider {
            base_path: dir.path().to_str().unwrap().to_string(),
        };

        let results = provider.search("microservices").unwrap();
        assert!(!results.is_empty());
        assert!(results[0].contains("Skill"));
    }

    #[test]
    fn test_file_provider_search_companies() {
        let dir = setup_test_kb();
        let provider = FileKnowledgeProvider {
            base_path: dir.path().to_str().unwrap().to_string(),
        };

        let results = provider.search("acme").unwrap();
        assert!(!results.is_empty());
        assert!(results[0].contains("Company Research"));
    }

    #[test]
    fn test_file_provider_search_no_results() {
        let dir = setup_test_kb();
        let provider = FileKnowledgeProvider {
            base_path: dir.path().to_str().unwrap().to_string(),
        };

        let results = provider
            .search("quantum computing blockchain web3")
            .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_file_provider_save_observation() {
        let dir = setup_test_kb();
        let provider = FileKnowledgeProvider {
            base_path: dir.path().to_str().unwrap().to_string(),
        };

        provider
            .save_observation("Test Session", "# Test Content\nSome notes.")
            .unwrap();

        let saved_path = dir.path().join("interviews").join("Test_Session.md");
        assert!(saved_path.exists());
        let content = fs::read_to_string(&saved_path).unwrap();
        assert!(content.contains("Test Content"));
    }

    // ========================
    // KnowledgeManager Tests
    // ========================

    #[test]
    fn test_knowledge_manager_load_profile() {
        let dir = setup_test_kb();
        let mut km = KnowledgeManager::new(dir.path().to_str().unwrap());

        km.load_personal_profile().unwrap();
        assert!(km.personal_profile.is_some());
        assert_eq!(km.personal_profile.as_ref().unwrap().nombre, "Test User");
    }

    #[test]
    fn test_knowledge_manager_search_all() {
        let dir = setup_test_kb();
        let km = KnowledgeManager::new(dir.path().to_str().unwrap());

        let results = km.search_all("Kubernetes");
        assert!(!results.is_empty());
        // Should find results from both skills and companies
        let has_skill = results.iter().any(|r| r.contains("Skill"));
        let has_company = results.iter().any(|r| r.contains("Company"));
        assert!(has_skill || has_company);
    }

    #[test]
    fn test_knowledge_manager_search_all_aggregates_providers() {
        let dir = setup_test_kb();
        let km = KnowledgeManager::new(dir.path().to_str().unwrap());

        let results = km.search_all("Go");
        assert!(!results.is_empty());
        // Results should be prefixed with provider name
        assert!(results.iter().all(|r| r.starts_with('[')));
    }

    // ========================
    // System Prompt Generation Tests
    // ========================

    #[test]
    fn test_generate_system_prompt_with_profile() {
        let dir = setup_test_kb();
        let mut km = KnowledgeManager::new(dir.path().to_str().unwrap());
        km.load_personal_profile().unwrap();

        let prompt = generate_system_prompt(&km);
        assert!(prompt.contains("Pelendur"));
        assert!(prompt.contains("Test User"));
        assert!(prompt.contains("Senior Backend Engineer"));
        assert!(prompt.contains("Go"));
        assert!(prompt.contains("STAR"));
    }

    #[test]
    fn test_generate_system_prompt_without_profile() {
        let dir = setup_test_kb();
        let km = KnowledgeManager::new(dir.path().to_str().unwrap());

        let prompt = generate_system_prompt(&km);
        assert!(prompt.contains("Pelendur"));
        assert!(!prompt.contains("Test User"));
        assert!(prompt.contains("STAR")); // STAR template always present
    }
}
