//! CV Upload + AI Parsing Pipeline
//!
//! Extracts text from PDF or plain-text CVs, sends the content to the LLM
//! for structured extraction, and inserts the parsed entities into the
//! SQLite knowledge graph with deduplication.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::config::Config;
use crate::llm::ChatMessage;

// ---------------------------------------------------------------------------
// Structured output types
// ---------------------------------------------------------------------------

/// Top-level structured output from LLM CV parsing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedCv {
    pub skills: Vec<ParsedSkill>,
    pub experiences: Vec<ParsedExperience>,
    pub projects: Vec<ParsedProject>,
    pub education: Vec<ParsedEducation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSkill {
    pub name: String,
    pub category: Option<String>,
    pub level: String,
    pub years: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedExperience {
    pub company: String,
    pub role: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub description: Option<String>,
    pub highlights: Option<String>,
    pub skills_used: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedProject {
    pub name: String,
    pub description: Option<String>,
    pub url: Option<String>,
    pub keywords: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedEducation {
    pub institution: String,
    pub degree: String,
    pub field: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

/// Summary returned after importing parsed CV data into the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CvImportResult {
    pub skills_created: usize,
    pub skills_merged: usize,
    pub experiences_created: usize,
    pub projects_created: usize,
    pub edges_created: usize,
    pub education_saved: usize,
}

// ---------------------------------------------------------------------------
// Text extraction
// ---------------------------------------------------------------------------

/// Extract text content from a CV file (PDF or plain text).
pub fn extract_text(file_path: &Path) -> Result<String> {
    let extension = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        "pdf" => {
            let bytes = std::fs::read(file_path)
                .with_context(|| format!("Failed to read PDF file: {:?}", file_path))?;
            let text = pdf_extract::extract_text_from_mem(&bytes)
                .context("Failed to extract text from PDF")?;
            Ok(text)
        }
        "txt" | "md" | "" => {
            let text = std::fs::read_to_string(file_path)
                .with_context(|| format!("Failed to read text file: {:?}", file_path))?;
            Ok(text)
        }
        _ => anyhow::bail!(
            "Unsupported file type: '{}'. Only PDF and plain text files are supported.",
            extension
        ),
    }
}

// ---------------------------------------------------------------------------
// LLM prompt construction
// ---------------------------------------------------------------------------

/// Build the system + user messages for CV extraction.
fn build_extraction_prompt(cv_text: &str) -> Vec<ChatMessage> {
    let system_prompt = r#"You are a CV/Resume parsing assistant. Extract structured data from the CV text provided.

IMPORTANT RULES:
1. Respond ONLY with valid JSON. No markdown fences, no explanation text.
2. Handle both Spanish and English CVs — detect language automatically.
3. For skill levels, use ONE of: "expert", "advanced", "intermediate", "learning".
4. For dates, use YYYY-MM format when possible, or the format found in the CV.
5. The "highlights" field should be a JSON array string like "["Achievement A","Achievement B"]".
6. The "skills_used" field lists skill names that were used in that experience (for creating graph edges).
7. If a field is unknown or not present, use null.
8. Extract ALL skills mentioned anywhere in the CV (skills section, experience descriptions, projects).
9. For education, extract institution, degree, field of study, and dates.

Output this exact JSON structure:
{
  "skills": [{"name": "...", "category": "...", "level": "...", "years": N}],
  "experiences": [{"company": "...", "role": "...", "start_date": "...", "end_date": "...", "description": "...", "highlights": "...", "skills_used": ["..."]}],
  "projects": [{"name": "...", "description": "...", "url": "...", "keywords": "...", "start_date": "...", "end_date": "..."}],
  "education": [{"institution": "...", "degree": "...", "field": "...", "start_date": "...", "end_date": "..."}]
}"#;

    vec![
        ChatMessage {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: format!("Parse this CV:\n\n{}", cv_text),
        },
    ]
}

// ---------------------------------------------------------------------------
// LLM call + JSON parsing
// ---------------------------------------------------------------------------

/// Parse CV text through the LLM and return structured data.
pub async fn parse_cv_with_llm(config: &Config, cv_text: &str) -> Result<ParsedCv> {
    if cv_text.trim().is_empty() {
        anyhow::bail!("CV text is empty — nothing to parse");
    }

    let messages = build_extraction_prompt(cv_text);

    // CV parsing needs more tokens than the default 500
    let response = crate::llm::generate_response_with_options(config, &messages, 4000)
        .await
        .context("LLM call failed during CV parsing")?;

    // Strip markdown fences if the model added them despite instructions
    let json_str = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let parsed: ParsedCv = serde_json::from_str(json_str).with_context(|| {
        format!(
            "Failed to parse LLM response as structured CV data. Response preview: {}",
            &json_str[..json_str.len().min(500)]
        )
    })?;

    Ok(parsed)
}

// ---------------------------------------------------------------------------
// Graph insertion with deduplication
// ---------------------------------------------------------------------------

use super::graph::{EntityType, KnowledgeGraph};

/// Insert parsed CV data into the knowledge graph with deduplication.
///
/// Returns a summary of what was created versus merged.
pub fn import_parsed_cv(graph: &KnowledgeGraph, parsed: &ParsedCv) -> Result<CvImportResult> {
    let mut result = CvImportResult {
        skills_created: 0,
        skills_merged: 0,
        experiences_created: 0,
        projects_created: 0,
        edges_created: 0,
        education_saved: 0,
    };

    // Build existing skill name→id map for deduplication (case-insensitive)
    let mut skill_name_to_id: HashMap<String, String> = HashMap::new();
    for skill in graph.list_skills()? {
        skill_name_to_id
            .entry(skill.name.to_lowercase())
            .or_insert(skill.id);
    }

    // 1. Insert skills (deduplicate by name, case-insensitive)
    for skill in &parsed.skills {
        let key = skill.name.to_lowercase();
        if skill_name_to_id.contains_key(&key) {
            result.skills_merged += 1;
        } else {
            let entity = graph.create_skill(
                &skill.name,
                skill.category.as_deref(),
                &skill.level,
                skill.years,
                Some("cv_upload"),
            )?;
            skill_name_to_id.insert(key, entity.id);
            result.skills_created += 1;
        }
    }

    // 2. Insert experiences, track (exp_id, skill_names) for edge creation
    let mut experience_links: Vec<(String, Vec<String>)> = Vec::new();

    for exp in &parsed.experiences {
        let entity = graph.create_experience(
            &exp.company,
            &exp.role,
            &exp.start_date,
            exp.end_date.as_deref(),
            exp.description.as_deref(),
            exp.highlights.as_deref(),
        )?;
        experience_links.push((entity.id, exp.skills_used.clone()));
        result.experiences_created += 1;
    }

    // 3. Insert projects
    for proj in &parsed.projects {
        graph.create_project(
            &proj.name,
            proj.description.as_deref(),
            proj.url.as_deref(),
            proj.keywords.as_deref(),
            proj.start_date.as_deref(),
            proj.end_date.as_deref(),
        )?;
        result.projects_created += 1;
    }

    // 4. Create edges: skill → experience ("used_in")
    for (exp_id, skill_names) in &experience_links {
        for skill_name in skill_names {
            let key = skill_name.to_lowercase();
            if let Some(skill_id) = skill_name_to_id.get(&key) {
                if !edge_exists(graph, skill_id, exp_id, "used_in") {
                    let _ = graph.add_edge(
                        skill_id,
                        EntityType::Skill,
                        exp_id,
                        EntityType::Experience,
                        "used_in",
                        1.0,
                    );
                    result.edges_created += 1;
                }
            }
        }
    }

    // 5. Save education as metadata entries (no dedicated education table)
    for (i, edu) in parsed.education.iter().enumerate() {
        let key = format!("education:{}", i);
        if graph.get_metadata(&key)?.is_none() {
            let value = format!(
                "{}|{}|{}|{}|{}",
                edu.institution,
                edu.degree,
                edu.field.as_deref().unwrap_or(""),
                edu.start_date.as_deref().unwrap_or(""),
                edu.end_date.as_deref().unwrap_or("")
            );
            graph.set_metadata(&key, &value)?;
            result.education_saved += 1;
        }
    }

    Ok(result)
}

/// Check if an edge already exists between two entities with a given relation.
/// Mirrors the same pattern as `migration.rs`.
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::graph::KnowledgeGraph;

    #[test]
    fn test_build_extraction_prompt() {
        let messages = build_extraction_prompt("John Doe - Software Engineer");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "system");
        assert!(messages[1].content.contains("John Doe"));
        assert!(messages[0].content.contains("JSON"));
    }

    #[test]
    fn test_parse_llm_json_response() {
        let json = r#"{
            "skills": [{"name": "Go", "category": "backend", "level": "expert", "years": 5}],
            "experiences": [{"company": "Acme", "role": "Engineer", "start_date": "2020-01", "end_date": null, "description": "Built things", "highlights": null, "skills_used": ["Go"]}],
            "projects": [],
            "education": [{"institution": "MIT", "degree": "BSc CS", "field": "Computer Science", "start_date": "2010", "end_date": "2014"}]
        }"#;
        let parsed: ParsedCv = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.skills.len(), 1);
        assert_eq!(parsed.skills[0].name, "Go");
        assert_eq!(parsed.experiences.len(), 1);
        assert_eq!(parsed.education.len(), 1);
    }

    #[test]
    fn test_strips_markdown_fences() {
        let raw =
            "```json\n{\"skills\":[],\"experiences\":[],\"projects\":[],\"education\":[]}\n```";
        let stripped = raw
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        let parsed: ParsedCv = serde_json::from_str(stripped).unwrap();
        assert!(parsed.skills.is_empty());
    }

    #[test]
    fn test_import_deduplication() {
        let graph = KnowledgeGraph::open_in_memory().unwrap();

        // Pre-existing skill
        let _existing =
            graph.create_skill("Go", Some("backend"), "expert", 5, Some("profile.yaml"));

        let parsed = ParsedCv {
            skills: vec![
                ParsedSkill {
                    name: "Go".to_string(),
                    category: Some("backend".to_string()),
                    level: "expert".to_string(),
                    years: 5,
                },
                ParsedSkill {
                    name: "Rust".to_string(),
                    category: Some("backend".to_string()),
                    level: "learning".to_string(),
                    years: 1,
                },
            ],
            experiences: vec![ParsedExperience {
                company: "Acme".to_string(),
                role: "Engineer".to_string(),
                start_date: "2020-01".to_string(),
                end_date: None,
                description: Some("Built things".to_string()),
                highlights: None,
                skills_used: vec!["Go".to_string(), "Rust".to_string()],
            }],
            projects: vec![],
            education: vec![],
        };

        let result = import_parsed_cv(&graph, &parsed).unwrap();

        assert_eq!(result.skills_merged, 1);
        assert_eq!(result.skills_created, 1);
        assert_eq!(result.experiences_created, 1);
        assert_eq!(result.edges_created, 2);

        let all_skills = graph.list_skills().unwrap();
        assert_eq!(all_skills.len(), 2);
    }

    #[test]
    fn test_import_education_as_metadata() {
        let graph = KnowledgeGraph::open_in_memory().unwrap();

        let parsed = ParsedCv {
            skills: vec![],
            experiences: vec![],
            projects: vec![],
            education: vec![ParsedEducation {
                institution: "MIT".to_string(),
                degree: "BSc Computer Science".to_string(),
                field: Some("CS".to_string()),
                start_date: Some("2010".to_string()),
                end_date: Some("2014".to_string()),
            }],
        };

        let result = import_parsed_cv(&graph, &parsed).unwrap();
        assert_eq!(result.education_saved, 1);

        let stored = graph.get_metadata("education:0").unwrap().unwrap();
        assert!(stored.contains("MIT"));
        assert!(stored.contains("BSc Computer Science"));
    }

    #[test]
    fn test_import_idempotent_education() {
        let graph = KnowledgeGraph::open_in_memory().unwrap();

        let parsed = ParsedCv {
            skills: vec![],
            experiences: vec![],
            projects: vec![],
            education: vec![ParsedEducation {
                institution: "MIT".to_string(),
                degree: "BSc".to_string(),
                field: None,
                start_date: None,
                end_date: None,
            }],
        };

        let r1 = import_parsed_cv(&graph, &parsed).unwrap();
        assert_eq!(r1.education_saved, 1);

        let r2 = import_parsed_cv(&graph, &parsed).unwrap();
        assert_eq!(r2.education_saved, 0);
    }

    #[test]
    fn test_unsupported_file_type() {
        let result = extract_text(Path::new("resume.docx"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[test]
    fn test_case_insensitive_skill_dedup() {
        let graph = KnowledgeGraph::open_in_memory().unwrap();

        // Pre-existing with different casing
        let _existing = graph.create_skill("docker", None, "expert", 3, None);

        let parsed = ParsedCv {
            skills: vec![ParsedSkill {
                name: "Docker".to_string(), // different case
                category: Some("devops".to_string()),
                level: "expert".to_string(),
                years: 3,
            }],
            experiences: vec![],
            projects: vec![],
            education: vec![],
        };

        let result = import_parsed_cv(&graph, &parsed).unwrap();
        assert_eq!(result.skills_merged, 1);
        assert_eq!(result.skills_created, 0);
    }
}
