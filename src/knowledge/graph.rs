//! SQLite Knowledge Graph — entity storage + relationship edges.
//!
//! Stores skills, experiences, projects, companies, and STAR stories as
//! first-class entities.  Relationships are tracked via a generic `edges`
//! table so any two entities can be linked (e.g. skill→project, story→company).

use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::embeddings::{PersistentVectorStore, SemanticSearchResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEntity {
    pub id: String,
    pub name: String,
    pub category: Option<String>,
    pub level: String,
    pub years: i32,
    pub source: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceEntity {
    pub id: String,
    pub company: String,
    pub role: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub description: Option<String>,
    pub highlights: Option<String>, // JSON array stored as text
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectEntity {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub url: Option<String>,
    pub keywords: Option<String>, // comma-separated
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyEntity {
    pub id: String,
    pub name: String,
    pub industry: Option<String>,
    pub description: Option<String>,
    pub culture: Option<String>,
    pub tech_stack: Option<String>, // comma-separated
    pub strategic_angle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarStoryEntity {
    pub id: String,
    pub title: Option<String>,
    pub situation: String,
    pub task: String,
    pub action: String,
    pub result: String,
    pub tags: Option<String>, // JSON array stored as text
    pub difficulty: Option<String>,
    pub stakes: Option<String>,
    pub usage_count: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: String,
    pub source_id: String,
    pub source_type: String,
    pub target_id: String,
    pub target_type: String,
    pub relation: String,
    pub weight: f64,
}

/// Entity types that can participate in edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityType {
    Skill,
    Experience,
    Project,
    Company,
    StarStory,
}

impl EntityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EntityType::Skill => "skill",
            EntityType::Experience => "experience",
            EntityType::Project => "project",
            EntityType::Company => "company",
            EntityType::StarStory => "star_story",
        }
    }
}

pub struct KnowledgeGraph {
    conn: Connection,
}

impl KnowledgeGraph {
    /// Open (or create) the SQLite database at `path` and ensure all tables exist.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open SQLite DB at {:?}", path))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .context("Failed to set pragmas")?;
        let kg = Self { conn };
        kg.ensure_tables_exist()?;
        Ok(kg)
    }

    /// Open an in-memory database (useful for tests).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        let kg = Self { conn };
        kg.ensure_tables_exist()?;
        Ok(kg)
    }

    pub fn ensure_tables_exist(&self) -> Result<()> {
        self.conn
            .execute_batch(
                "
                CREATE TABLE IF NOT EXISTS skills (
                    id         TEXT PRIMARY KEY,
                    name       TEXT NOT NULL,
                    category   TEXT,
                    level      TEXT NOT NULL,
                    years      INTEGER NOT NULL DEFAULT 0,
                    source     TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS experiences (
                    id          TEXT PRIMARY KEY,
                    company     TEXT NOT NULL,
                    role        TEXT NOT NULL,
                    start_date  TEXT NOT NULL,
                    end_date    TEXT,
                    description TEXT,
                    highlights  TEXT
                );

                CREATE TABLE IF NOT EXISTS projects (
                    id          TEXT PRIMARY KEY,
                    name        TEXT NOT NULL,
                    description TEXT,
                    url         TEXT,
                    keywords    TEXT,
                    start_date  TEXT,
                    end_date    TEXT
                );

                CREATE TABLE IF NOT EXISTS companies (
                    id              TEXT PRIMARY KEY,
                    name            TEXT NOT NULL,
                    industry        TEXT,
                    description     TEXT,
                    culture         TEXT,
                    tech_stack      TEXT,
                    strategic_angle TEXT
                );

                CREATE TABLE IF NOT EXISTS star_stories (
                    id          TEXT PRIMARY KEY,
                    title       TEXT,
                    situation   TEXT NOT NULL,
                    task        TEXT NOT NULL,
                    action      TEXT NOT NULL,
                    result      TEXT NOT NULL,
                    tags        TEXT,
                    difficulty  TEXT,
                    stakes      TEXT,
                    usage_count INTEGER NOT NULL DEFAULT 0,
                    created_at  TEXT NOT NULL,
                    updated_at  TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS edges (
                    id          TEXT PRIMARY KEY,
                    source_id   TEXT NOT NULL,
                    source_type TEXT NOT NULL,
                    target_id   TEXT NOT NULL,
                    target_type TEXT NOT NULL,
                    relation    TEXT NOT NULL,
                    weight      REAL NOT NULL DEFAULT 1.0
                );

                CREATE TABLE IF NOT EXISTS metadata (
                    key   TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );

                CREATE INDEX IF NOT EXISTS idx_edges_source ON edges(source_id, source_type);
                CREATE INDEX IF NOT EXISTS idx_edges_target ON edges(target_id, target_type);
                CREATE INDEX IF NOT EXISTS idx_skills_name   ON skills(name);
                CREATE INDEX IF NOT EXISTS idx_star_stories_tags ON star_stories(tags);
                ",
            )
            .context("Failed to create knowledge graph tables")?;

        if let Err(e) = super::search::ensure_fts_tables(&self.conn) {
            tracing::warn!("FTS5 tables not available (non-critical): {}", e);
        }

        Ok(())
    }

    fn now_iso() -> String {
        Utc::now().to_rfc3339()
    }

    fn new_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    pub fn create_skill(
        &self,
        name: &str,
        category: Option<&str>,
        level: &str,
        years: i32,
        source: Option<&str>,
    ) -> Result<SkillEntity> {
        let id = Self::new_id();
        let now = Self::now_iso();
        self.conn.execute(
            "INSERT INTO skills (id, name, category, level, years, source, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, name, category, level, years, source, now, now],
        )?;
        Ok(SkillEntity {
            id,
            name: name.to_string(),
            category: category.map(String::from),
            level: level.to_string(),
            years,
            source: source.map(String::from),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn get_skill(&self, id: &str) -> Result<Option<SkillEntity>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, category, level, years, source, created_at, updated_at FROM skills WHERE id = ?1")?;
        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(SkillEntity {
                id: row.get(0)?,
                name: row.get(1)?,
                category: row.get(2)?,
                level: row.get(3)?,
                years: row.get(4)?,
                source: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })),
            None => Ok(None),
        }
    }

    pub fn update_skill(&self, entity: &SkillEntity) -> Result<()> {
        let now = Self::now_iso();
        self.conn.execute(
            "UPDATE skills SET name=?1, category=?2, level=?3, years=?4, source=?5, updated_at=?6 WHERE id=?7",
            params![entity.name, entity.category, entity.level, entity.years, entity.source, now, entity.id],
        )?;
        Ok(())
    }

    pub fn delete_skill(&self, id: &str) -> Result<bool> {
        let affected = self
            .conn
            .execute("DELETE FROM skills WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    pub fn list_skills(&self) -> Result<Vec<SkillEntity>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, category, level, years, source, created_at, updated_at FROM skills ORDER BY name")?;
        let rows = stmt.query_map([], |row| {
            Ok(SkillEntity {
                id: row.get(0)?,
                name: row.get(1)?,
                category: row.get(2)?,
                level: row.get(3)?,
                years: row.get(4)?,
                source: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn create_experience(
        &self,
        company: &str,
        role: &str,
        start_date: &str,
        end_date: Option<&str>,
        description: Option<&str>,
        highlights: Option<&str>,
    ) -> Result<ExperienceEntity> {
        let id = Self::new_id();
        self.conn.execute(
            "INSERT INTO experiences (id, company, role, start_date, end_date, description, highlights)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, company, role, start_date, end_date, description, highlights],
        )?;
        Ok(ExperienceEntity {
            id,
            company: company.to_string(),
            role: role.to_string(),
            start_date: start_date.to_string(),
            end_date: end_date.map(String::from),
            description: description.map(String::from),
            highlights: highlights.map(String::from),
        })
    }

    pub fn get_experience(&self, id: &str) -> Result<Option<ExperienceEntity>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company, role, start_date, end_date, description, highlights FROM experiences WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(ExperienceEntity {
                id: row.get(0)?,
                company: row.get(1)?,
                role: row.get(2)?,
                start_date: row.get(3)?,
                end_date: row.get(4)?,
                description: row.get(5)?,
                highlights: row.get(6)?,
            })),
            None => Ok(None),
        }
    }

    pub fn update_experience(&self, entity: &ExperienceEntity) -> Result<()> {
        self.conn.execute(
            "UPDATE experiences SET company=?1, role=?2, start_date=?3, end_date=?4, description=?5, highlights=?6 WHERE id=?7",
            params![entity.company, entity.role, entity.start_date, entity.end_date, entity.description, entity.highlights, entity.id],
        )?;
        Ok(())
    }

    pub fn delete_experience(&self, id: &str) -> Result<bool> {
        let affected = self
            .conn
            .execute("DELETE FROM experiences WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    pub fn list_experiences(&self) -> Result<Vec<ExperienceEntity>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company, role, start_date, end_date, description, highlights FROM experiences ORDER BY start_date DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ExperienceEntity {
                id: row.get(0)?,
                company: row.get(1)?,
                role: row.get(2)?,
                start_date: row.get(3)?,
                end_date: row.get(4)?,
                description: row.get(5)?,
                highlights: row.get(6)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn create_project(
        &self,
        name: &str,
        description: Option<&str>,
        url: Option<&str>,
        keywords: Option<&str>,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> Result<ProjectEntity> {
        let id = Self::new_id();
        self.conn.execute(
            "INSERT INTO projects (id, name, description, url, keywords, start_date, end_date)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, name, description, url, keywords, start_date, end_date],
        )?;
        Ok(ProjectEntity {
            id,
            name: name.to_string(),
            description: description.map(String::from),
            url: url.map(String::from),
            keywords: keywords.map(String::from),
            start_date: start_date.map(String::from),
            end_date: end_date.map(String::from),
        })
    }

    pub fn get_project(&self, id: &str) -> Result<Option<ProjectEntity>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, url, keywords, start_date, end_date FROM projects WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(ProjectEntity {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                url: row.get(3)?,
                keywords: row.get(4)?,
                start_date: row.get(5)?,
                end_date: row.get(6)?,
            })),
            None => Ok(None),
        }
    }

    pub fn update_project(&self, entity: &ProjectEntity) -> Result<()> {
        self.conn.execute(
            "UPDATE projects SET name=?1, description=?2, url=?3, keywords=?4, start_date=?5, end_date=?6 WHERE id=?7",
            params![entity.name, entity.description, entity.url, entity.keywords, entity.start_date, entity.end_date, entity.id],
        )?;
        Ok(())
    }

    pub fn delete_project(&self, id: &str) -> Result<bool> {
        let affected = self
            .conn
            .execute("DELETE FROM projects WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    pub fn list_projects(&self) -> Result<Vec<ProjectEntity>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, url, keywords, start_date, end_date FROM projects ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ProjectEntity {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                url: row.get(3)?,
                keywords: row.get(4)?,
                start_date: row.get(5)?,
                end_date: row.get(6)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn create_company(
        &self,
        name: &str,
        industry: Option<&str>,
        description: Option<&str>,
        culture: Option<&str>,
        tech_stack: Option<&str>,
        strategic_angle: Option<&str>,
    ) -> Result<CompanyEntity> {
        let id = Self::new_id();
        self.conn.execute(
            "INSERT INTO companies (id, name, industry, description, culture, tech_stack, strategic_angle)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, name, industry, description, culture, tech_stack, strategic_angle],
        )?;
        Ok(CompanyEntity {
            id,
            name: name.to_string(),
            industry: industry.map(String::from),
            description: description.map(String::from),
            culture: culture.map(String::from),
            tech_stack: tech_stack.map(String::from),
            strategic_angle: strategic_angle.map(String::from),
        })
    }

    pub fn get_company(&self, id: &str) -> Result<Option<CompanyEntity>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, industry, description, culture, tech_stack, strategic_angle FROM companies WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(CompanyEntity {
                id: row.get(0)?,
                name: row.get(1)?,
                industry: row.get(2)?,
                description: row.get(3)?,
                culture: row.get(4)?,
                tech_stack: row.get(5)?,
                strategic_angle: row.get(6)?,
            })),
            None => Ok(None),
        }
    }

    pub fn update_company(&self, entity: &CompanyEntity) -> Result<()> {
        self.conn.execute(
            "UPDATE companies SET name=?1, industry=?2, description=?3, culture=?4, tech_stack=?5, strategic_angle=?6 WHERE id=?7",
            params![entity.name, entity.industry, entity.description, entity.culture, entity.tech_stack, entity.strategic_angle, entity.id],
        )?;
        Ok(())
    }

    pub fn delete_company(&self, id: &str) -> Result<bool> {
        let affected = self
            .conn
            .execute("DELETE FROM companies WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    pub fn list_companies(&self) -> Result<Vec<CompanyEntity>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, industry, description, culture, tech_stack, strategic_angle FROM companies ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(CompanyEntity {
                id: row.get(0)?,
                name: row.get(1)?,
                industry: row.get(2)?,
                description: row.get(3)?,
                culture: row.get(4)?,
                tech_stack: row.get(5)?,
                strategic_angle: row.get(6)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn create_star_story(
        &self,
        title: Option<&str>,
        situation: &str,
        task: &str,
        action: &str,
        result: &str,
        tags: Option<&str>,
        difficulty: Option<&str>,
        stakes: Option<&str>,
    ) -> Result<StarStoryEntity> {
        let id = Self::new_id();
        let now = Self::now_iso();
        self.conn.execute(
            "INSERT INTO star_stories (id, title, situation, task, action, result, tags, difficulty, stakes, usage_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, ?10, ?11)",
            params![id, title, situation, task, action, result, tags, difficulty, stakes, now, now],
        )?;
        Ok(StarStoryEntity {
            id,
            title: title.map(String::from),
            situation: situation.to_string(),
            task: task.to_string(),
            action: action.to_string(),
            result: result.to_string(),
            tags: tags.map(String::from),
            difficulty: difficulty.map(String::from),
            stakes: stakes.map(String::from),
            usage_count: 0,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn get_star_story(&self, id: &str) -> Result<Option<StarStoryEntity>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, situation, task, action, result, tags, difficulty, stakes, usage_count, created_at, updated_at FROM star_stories WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(StarStoryEntity {
                id: row.get(0)?,
                title: row.get(1)?,
                situation: row.get(2)?,
                task: row.get(3)?,
                action: row.get(4)?,
                result: row.get(5)?,
                tags: row.get(6)?,
                difficulty: row.get(7)?,
                stakes: row.get(8)?,
                usage_count: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })),
            None => Ok(None),
        }
    }

    pub fn update_star_story(&self, entity: &StarStoryEntity) -> Result<()> {
        let now = Self::now_iso();
        self.conn.execute(
            "UPDATE star_stories SET title=?1, situation=?2, task=?3, action=?4, result=?5, tags=?6, difficulty=?7, stakes=?8, usage_count=?9, updated_at=?10 WHERE id=?11",
            params![entity.title, entity.situation, entity.task, entity.action, entity.result, entity.tags, entity.difficulty, entity.stakes, entity.usage_count, now, entity.id],
        )?;
        Ok(())
    }

    pub fn delete_star_story(&self, id: &str) -> Result<bool> {
        let affected = self
            .conn
            .execute("DELETE FROM star_stories WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    pub fn list_star_stories(&self) -> Result<Vec<StarStoryEntity>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, situation, task, action, result, tags, difficulty, stakes, usage_count, created_at, updated_at FROM star_stories ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(StarStoryEntity {
                id: row.get(0)?,
                title: row.get(1)?,
                situation: row.get(2)?,
                task: row.get(3)?,
                action: row.get(4)?,
                result: row.get(5)?,
                tags: row.get(6)?,
                difficulty: row.get(7)?,
                stakes: row.get(8)?,
                usage_count: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn add_edge(
        &self,
        source_id: &str,
        source_type: EntityType,
        target_id: &str,
        target_type: EntityType,
        relation: &str,
        weight: f64,
    ) -> Result<Edge> {
        let id = Self::new_id();
        self.conn.execute(
            "INSERT INTO edges (id, source_id, source_type, target_id, target_type, relation, weight)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                source_id,
                source_type.as_str(),
                target_id,
                target_type.as_str(),
                relation,
                weight
            ],
        )?;
        Ok(Edge {
            id,
            source_id: source_id.to_string(),
            source_type: source_type.as_str().to_string(),
            target_id: target_id.to_string(),
            target_type: target_type.as_str().to_string(),
            relation: relation.to_string(),
            weight,
        })
    }

    pub fn remove_edge(&self, edge_id: &str) -> Result<bool> {
        let affected = self
            .conn
            .execute("DELETE FROM edges WHERE id = ?1", params![edge_id])?;
        Ok(affected > 0)
    }

    pub fn get_edges_for_entity(
        &self,
        entity_id: &str,
        entity_type: EntityType,
    ) -> Result<Vec<Edge>> {
        let type_str = entity_type.as_str();
        let mut stmt = self.conn.prepare(
            "SELECT id, source_id, source_type, target_id, target_type, relation, weight
             FROM edges
             WHERE (source_id = ?1 AND source_type = ?2)
                OR (target_id = ?1 AND target_type = ?2)",
        )?;
        let rows = stmt.query_map(params![entity_id, type_str], |row| {
            Ok(Edge {
                id: row.get(0)?,
                source_id: row.get(1)?,
                source_type: row.get(2)?,
                target_id: row.get(3)?,
                target_type: row.get(4)?,
                relation: row.get(5)?,
                weight: row.get(6)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn set_metadata(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_metadata(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM metadata WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }

    pub fn list_all_edges(&self) -> Result<Vec<Edge>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_id, source_type, target_id, target_type, relation, weight FROM edges",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Edge {
                id: row.get(0)?,
                source_id: row.get(1)?,
                source_type: row.get(2)?,
                target_id: row.get(3)?,
                target_type: row.get(4)?,
                relation: row.get(5)?,
                weight: row.get(6)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn semantic_search(&self, query: &str, top_k: usize) -> Result<Vec<SemanticSearchResult>> {
        let store = PersistentVectorStore::new(&self.conn);
        store
            .init_tables()
            .map_err(|e| anyhow::anyhow!("Failed to init embeddings: {}", e))?;
        store
            .search(query, top_k)
            .map_err(|e| anyhow::anyhow!("Search failed: {}", e))
    }

    pub fn generate_embeddings(&self) -> Result<()> {
        let store = PersistentVectorStore::new(&self.conn);
        store
            .init_tables()
            .map_err(|e| anyhow::anyhow!("Failed to init embeddings: {}", e))?;
        store
            .generate_all_embeddings(&self.conn)
            .map_err(|e| anyhow::anyhow!("Failed to generate embeddings: {}", e))?;
        Ok(())
    }

    pub fn refresh_entity_embedding(&self, entity_type: &str, entity_id: &str) -> Result<()> {
        let store = PersistentVectorStore::new(&self.conn);
        store
            .init_tables()
            .map_err(|e| anyhow::anyhow!("Failed to init embeddings: {}", e))?;

        match entity_type {
            "skill" => {
                if let Some(skill) = self.get_skill(entity_id)? {
                    let text = format!(
                        "{} {} {}",
                        skill.name,
                        skill.category.as_deref().unwrap_or_default(),
                        skill.level
                    );
                    let snippet = format!(
                        "{} ({}) - {}",
                        skill.name,
                        skill.level,
                        skill.category.as_deref().unwrap_or_default()
                    );
                    let service = super::embeddings::HashEmbeddingService::default();
                    let vector = service.embed(&text);
                    store
                        .upsert_embedding("skill", &skill.id, &skill.name, &snippet, &vector)
                        .map_err(|e| anyhow::anyhow!("Failed to upsert embedding: {}", e))?;
                }
            }
            "experience" => {
                if let Some(exp) = self.get_experience(entity_id)? {
                    let text = format!(
                        "{} {} {}",
                        exp.company,
                        exp.role,
                        exp.description.as_deref().unwrap_or_default()
                    );
                    let snippet = format!(
                        "{} at {} - {}",
                        exp.role,
                        exp.company,
                        exp.description.as_deref().unwrap_or_default()
                    );
                    let service = super::embeddings::HashEmbeddingService::default();
                    let vector = service.embed(&text);
                    store
                        .upsert_embedding("experience", &exp.id, &exp.role, &snippet, &vector)
                        .map_err(|e| anyhow::anyhow!("Failed to upsert embedding: {}", e))?;
                }
            }
            "project" => {
                if let Some(proj) = self.get_project(entity_id)? {
                    let text = format!(
                        "{} {} {}",
                        proj.name,
                        proj.description.as_deref().unwrap_or_default(),
                        proj.keywords.as_deref().unwrap_or_default()
                    );
                    let snippet = format!(
                        "{} - {}",
                        proj.name,
                        proj.description.as_deref().unwrap_or_default()
                    );
                    let service = super::embeddings::HashEmbeddingService::default();
                    let vector = service.embed(&text);
                    store
                        .upsert_embedding("project", &proj.id, &proj.name, &snippet, &vector)
                        .map_err(|e| anyhow::anyhow!("Failed to upsert embedding: {}", e))?;
                }
            }
            "company" => {
                if let Some(comp) = self.get_company(entity_id)? {
                    let text = format!(
                        "{} {} {} {}",
                        comp.name,
                        comp.industry.as_deref().unwrap_or_default(),
                        comp.description.as_deref().unwrap_or_default(),
                        comp.tech_stack.as_deref().unwrap_or_default()
                    );
                    let snippet = format!(
                        "{} - {}",
                        comp.name,
                        comp.description.as_deref().unwrap_or_default()
                    );
                    let service = super::embeddings::HashEmbeddingService::default();
                    let vector = service.embed(&text);
                    store
                        .upsert_embedding("company", &comp.id, &comp.name, &snippet, &vector)
                        .map_err(|e| anyhow::anyhow!("Failed to upsert embedding: {}", e))?;
                }
            }
            "star_story" => {
                if let Some(story) = self.get_star_story(entity_id)? {
                    let text = format!(
                        "{} {} {} {} {}",
                        story.title.as_deref().unwrap_or_default(),
                        story.situation,
                        story.task,
                        story.action,
                        story.result
                    );
                    let snippet = story.situation.clone();
                    let service = super::embeddings::HashEmbeddingService::default();
                    let vector = service.embed(&text);
                    store
                        .upsert_embedding(
                            "star_story",
                            &story.id,
                            &story.title.as_deref().unwrap_or(&story.situation),
                            &snippet,
                            &vector,
                        )
                        .map_err(|e| anyhow::anyhow!("Failed to upsert embedding: {}", e))?;
                }
            }
            _ => anyhow::bail!("Unknown entity type: {}", entity_type),
        }
        Ok(())
    }
}

use super::graph_search::GraphSearch;
use super::personal::KnowledgeProvider;
use std::sync::Mutex;

pub struct GraphKnowledgeProvider {
    graph: Mutex<KnowledgeGraph>,
}

impl GraphKnowledgeProvider {
    pub fn new(graph: KnowledgeGraph) -> Self {
        Self {
            graph: Mutex::new(graph),
        }
    }

    pub fn open(path: &Path) -> Result<Self> {
        let graph = KnowledgeGraph::open(path)?;
        Ok(Self {
            graph: Mutex::new(graph),
        })
    }

    pub fn open_in_memory() -> Result<Self> {
        let graph = KnowledgeGraph::open_in_memory()?;
        Ok(Self {
            graph: Mutex::new(graph),
        })
    }

    pub fn graph(&self) -> std::sync::MutexGuard<'_, KnowledgeGraph> {
        self.graph.lock().expect("KnowledgeGraph mutex poisoned")
    }
}

impl KnowledgeProvider for GraphKnowledgeProvider {
    fn get_name(&self) -> &str {
        "KnowledgeGraph"
    }

    fn search(&self, query: &str) -> Result<Vec<String>> {
        let graph = self.graph();
        let searcher = GraphSearch::new(&graph);
        let results = searcher.search_by_keyword(query)?;
        Ok(results
            .into_iter()
            .map(|r| format!("[{}] {} — {}", r.entity_type, r.matched_field, r.snippet))
            .collect())
    }

    fn save_observation(&self, title: &str, content: &str) -> Result<()> {
        let graph = self.graph();
        graph.set_metadata(&format!("observation:{}", title), content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> KnowledgeGraph {
        KnowledgeGraph::open_in_memory().unwrap()
    }

    #[test]
    fn test_all_tables_created() {
        let kg = setup();
        let tables: Vec<String> = kg
            .conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(tables.contains(&"skills".to_string()));
        assert!(tables.contains(&"experiences".to_string()));
        assert!(tables.contains(&"projects".to_string()));
        assert!(tables.contains(&"companies".to_string()));
        assert!(tables.contains(&"star_stories".to_string()));
        assert!(tables.contains(&"edges".to_string()));
        assert!(tables.contains(&"metadata".to_string()));
    }

    #[test]
    fn test_skill_crud() {
        let kg = setup();

        let skill = kg
            .create_skill("Go", Some("backend"), "expert", 5, None)
            .unwrap();
        assert_eq!(skill.name, "Go");
        assert_eq!(skill.level, "expert");

        let fetched = kg.get_skill(&skill.id).unwrap().unwrap();
        assert_eq!(fetched.name, "Go");
        assert_eq!(fetched.years, 5);

        let mut updated = fetched.clone();
        updated.years = 6;
        kg.update_skill(&updated).unwrap();
        let after_update = kg.get_skill(&skill.id).unwrap().unwrap();
        assert_eq!(after_update.years, 6);

        assert!(kg.delete_skill(&skill.id).unwrap());
        assert!(kg.get_skill(&skill.id).unwrap().is_none());
    }

    #[test]
    fn test_list_skills() {
        let kg = setup();
        kg.create_skill("Go", None, "expert", 5, None).unwrap();
        kg.create_skill("Rust", None, "learning", 1, None).unwrap();
        let list = kg.list_skills().unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_experience_crud() {
        let kg = setup();

        let exp = kg
            .create_experience(
                "Acme Corp",
                "Senior Engineer",
                "2020-01",
                None,
                Some("Built things"),
                None,
            )
            .unwrap();
        assert_eq!(exp.company, "Acme Corp");

        let fetched = kg.get_experience(&exp.id).unwrap().unwrap();
        assert_eq!(fetched.role, "Senior Engineer");

        let mut updated = fetched;
        updated.role = "Staff Engineer".to_string();
        kg.update_experience(&updated).unwrap();

        let after = kg.get_experience(&exp.id).unwrap().unwrap();
        assert_eq!(after.role, "Staff Engineer");

        assert!(kg.delete_experience(&exp.id).unwrap());
        assert!(kg.get_experience(&exp.id).unwrap().is_none());
    }

    #[test]
    fn test_project_crud() {
        let kg = setup();

        let proj = kg
            .create_project(
                "API Gateway",
                Some("High-perf gateway"),
                Some("https://example.com"),
                Some("go,microservices"),
                None,
                None,
            )
            .unwrap();
        assert_eq!(proj.name, "API Gateway");

        let fetched = kg.get_project(&proj.id).unwrap().unwrap();
        assert!(fetched.keywords.unwrap().contains("go"));

        assert!(kg.delete_project(&proj.id).unwrap());
    }

    #[test]
    fn test_company_crud() {
        let kg = setup();

        let comp = kg
            .create_company(
                "Acme",
                Some("Cloud"),
                Some("Cloud infra company"),
                None,
                Some("Go,K8s"),
                None,
            )
            .unwrap();
        assert_eq!(comp.name, "Acme");

        let fetched = kg.get_company(&comp.id).unwrap().unwrap();
        assert_eq!(fetched.industry.as_deref(), Some("Cloud"));

        assert!(kg.delete_company(&comp.id).unwrap());
    }

    #[test]
    fn test_star_story_crud() {
        let kg = setup();

        let story = kg
            .create_star_story(
                Some("Leadership crisis"),
                "Team lost lead",
                "Take ownership",
                "Organized standups",
                "Delivered early",
                Some(r#"["leadership","crisis"]"#),
                Some("medium"),
                Some("high"),
            )
            .unwrap();
        assert_eq!(story.situation, "Team lost lead");
        assert_eq!(story.usage_count, 0);

        let mut fetched = kg.get_star_story(&story.id).unwrap().unwrap();
        fetched.usage_count = 5;
        kg.update_star_story(&fetched).unwrap();

        let after = kg.get_star_story(&story.id).unwrap().unwrap();
        assert_eq!(after.usage_count, 5);

        assert!(kg.delete_star_story(&story.id).unwrap());
    }

    #[test]
    fn test_edge_management() {
        let kg = setup();

        let skill = kg.create_skill("Go", None, "expert", 5, None).unwrap();
        let proj = kg
            .create_project("Gateway", None, None, None, None, None)
            .unwrap();

        let edge = kg
            .add_edge(
                &skill.id,
                EntityType::Skill,
                &proj.id,
                EntityType::Project,
                "used_in",
                1.0,
            )
            .unwrap();
        assert_eq!(edge.relation, "used_in");

        let edges = kg
            .get_edges_for_entity(&skill.id, EntityType::Skill)
            .unwrap();
        assert_eq!(edges.len(), 1);

        assert!(kg.remove_edge(&edge.id).unwrap());
        let edges_after = kg
            .get_edges_for_entity(&skill.id, EntityType::Skill)
            .unwrap();
        assert!(edges_after.is_empty());
    }

    #[test]
    fn test_metadata() {
        let kg = setup();
        assert!(kg.get_metadata("version").unwrap().is_none());
        kg.set_metadata("version", "1.0").unwrap();
        assert_eq!(kg.get_metadata("version").unwrap().as_deref(), Some("1.0"));
        kg.set_metadata("version", "2.0").unwrap();
        assert_eq!(kg.get_metadata("version").unwrap().as_deref(), Some("2.0"));
    }

    #[test]
    fn test_query_performance_under_50ms() {
        let kg = setup();

        for i in 0..100 {
            kg.create_skill(
                &format!("Skill-{}", i),
                Some("test"),
                "intermediate",
                i % 10,
                None,
            )
            .unwrap();
        }

        let start = std::time::Instant::now();
        let _results = kg.list_skills().unwrap();
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 50,
            "list_skills took {}ms, expected < 50ms",
            elapsed.as_millis()
        );
    }
}
