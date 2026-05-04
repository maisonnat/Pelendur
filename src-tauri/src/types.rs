use serde::{Deserialize, Serialize};

// ─── Knowledge Graph Stats ─────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct KnowledgeGraphStats {
    pub skills: usize,
    pub experiences: usize,
    pub star_stories: usize,
    pub projects: usize,
    pub companies: usize,
}

// ─── Search Results ─────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct SearchResult {
    pub provider: String,
    pub content: String,
}

#[derive(Serialize, Clone)]
pub struct FuzzySearchResult {
    pub entity_type: String,
    pub entity_id: String,
    pub name: String,
    pub relevance_score: f64,
    pub match_type: String,
    pub matched_terms: Vec<String>,
}

#[derive(Serialize, Clone)]
pub struct EnhancedSearchResultIpc {
    pub entity_type: String,
    pub id: String,
    pub name: String,
    pub relevance_score: f64,
    pub matched_terms: Vec<String>,
    pub snippet: String,
}

#[derive(Serialize, Clone)]
pub struct SemanticSearchResult {
    pub entity_type: String,
    pub entity_id: String,
    pub name: String,
    pub similarity: f64,
    pub snippet: String,
}

// ─── Experience ─────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct ExperienceWithSkills {
    pub id: String,
    pub company: String,
    pub role: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub description: Option<String>,
    pub highlights: Option<String>,
    pub skills: Vec<String>,
}

#[derive(Serialize, Clone, Deserialize)]
pub struct ExperienceData {
    pub id: Option<String>,
    pub company: String,
    pub role: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub description: Option<String>,
    pub highlights: Option<String>,
    pub skill_ids: Option<Vec<String>>,
}

// ─── Skill ──────────────────────────────────────────────────────────────

#[derive(Serialize, Clone, Deserialize)]
pub struct SkillData {
    pub id: Option<String>,
    pub name: String,
    pub category: Option<String>,
    pub level: String,
    pub years: i32,
}

#[derive(Serialize, Clone)]
pub struct SkillRecord {
    pub id: String,
    pub name: String,
    pub category: Option<String>,
    pub level: String,
    pub years: i32,
    pub source: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ghostai_pilot::knowledge::graph::SkillEntity> for SkillRecord {
    fn from(e: ghostai_pilot::knowledge::graph::SkillEntity) -> Self {
        Self {
            id: e.id,
            name: e.name,
            category: e.category,
            level: e.level,
            years: e.years,
            source: e.source,
            created_at: e.created_at,
            updated_at: e.updated_at,
        }
    }
}

// ─── STAR Story ─────────────────────────────────────────────────────────

#[derive(Serialize, Clone, Deserialize)]
pub struct StarStoryData {
    pub id: Option<String>,
    pub title: Option<String>,
    pub situation: String,
    pub task: String,
    pub action: String,
    pub result: String,
    pub tags: Option<String>,
    pub difficulty: Option<String>,
    pub stakes: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct StarStoryRecord {
    pub id: String,
    pub title: Option<String>,
    pub situation: String,
    pub task: String,
    pub action: String,
    pub result: String,
    pub tags: Option<String>,
    pub difficulty: Option<String>,
    pub stakes: Option<String>,
    pub usage_count: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ghostai_pilot::knowledge::graph::StarStoryEntity> for StarStoryRecord {
    fn from(e: ghostai_pilot::knowledge::graph::StarStoryEntity) -> Self {
        Self {
            id: e.id,
            title: e.title,
            situation: e.situation,
            task: e.task,
            action: e.action,
            result: e.result,
            tags: e.tags,
            difficulty: e.difficulty,
            stakes: e.stakes,
            usage_count: e.usage_count,
            created_at: e.created_at,
            updated_at: e.updated_at,
        }
    }
}

// ─── Edges ──────────────────────────────────────────────────────────────

#[derive(Serialize, Clone, Deserialize)]
pub struct EdgeRecord {
    pub id: String,
    pub source_id: String,
    pub source_type: String,
    pub target_id: String,
    pub target_type: String,
    pub relation: String,
    pub weight: f64,
}

impl From<ghostai_pilot::knowledge::graph::Edge> for EdgeRecord {
    fn from(e: ghostai_pilot::knowledge::graph::Edge) -> Self {
        Self {
            id: e.id,
            source_id: e.source_id,
            source_type: e.source_type,
            target_id: e.target_id,
            target_type: e.target_type,
            relation: e.relation,
            weight: e.weight,
        }
    }
}

// ─── Company ────────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct CompanyRecord {
    pub id: String,
    pub name: String,
    pub industry: Option<String>,
    pub description: Option<String>,
}

impl From<ghostai_pilot::knowledge::graph::CompanyEntity> for CompanyRecord {
    fn from(e: ghostai_pilot::knowledge::graph::CompanyEntity) -> Self {
        Self {
            id: e.id,
            name: e.name,
            industry: e.industry,
            description: e.description,
        }
    }
}

// ─── Graph Data (full dump) ─────────────────────────────────────────────

#[derive(Serialize)]
pub struct GraphData {
    pub skills: Vec<SkillRecord>,
    pub experiences: Vec<ExperienceWithSkills>,
    pub star_stories: Vec<StarStoryRecord>,
    pub companies: Vec<CompanyRecord>,
    pub edges: Vec<EdgeRecord>,
}

// ─── CV ─────────────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct CvPreview {
    pub parsed: ghostai_pilot::knowledge::cv_parser::ParsedCv,
    pub file_name: String,
    pub text_length: usize,
}

// ─── Context / HUD ──────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct ContextResult {
    pub entity_type: String,
    pub name: String,
    pub relevance: f64,
    pub snippet: String,
}

#[derive(Serialize, Clone)]
pub struct StoryResult {
    pub id: String,
    pub title: String,
    pub tags: Option<String>,
    pub usage_count: i32,
    pub relevance: f64,
}

// ─── Interview Mode ─────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct InterviewSessionState {
    pub active: bool,
    pub company: Option<String>,
    pub started_at: Option<String>,
    pub duration_seconds: Option<u64>,
}

#[derive(Serialize, Clone)]
pub struct InterviewSummary {
    pub company: String,
    pub duration_seconds: u64,
    pub transcript_count: usize,
    pub summary_text: String,
    pub strengths: Vec<String>,
    pub areas_to_improve: Vec<String>,
    pub recommended_stories: Vec<String>,
}

#[derive(Serialize, Clone)]
pub struct CompanyInfo {
    pub name: String,
    pub industry: Option<String>,
    pub overview: String,
}

// ─── Helpers ────────────────────────────────────────────────────────────

pub fn parse_entity_type(s: &str) -> Result<ghostai_pilot::knowledge::graph::EntityType, String> {
    match s {
        "skill" => Ok(ghostai_pilot::knowledge::graph::EntityType::Skill),
        "experience" => Ok(ghostai_pilot::knowledge::graph::EntityType::Experience),
        "project" => Ok(ghostai_pilot::knowledge::graph::EntityType::Project),
        "company" => Ok(ghostai_pilot::knowledge::graph::EntityType::Company),
        "star_story" => Ok(ghostai_pilot::knowledge::graph::EntityType::StarStory),
        _ => Err(format!("Unknown entity type: {}", s)),
    }
}
