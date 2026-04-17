use crate::config::Config;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub entity_type: String,
    pub entity_id: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchResult {
    pub entity_type: String,
    pub entity_id: String,
    pub name: String,
    pub similarity: f64,
    pub snippet: String,
}

/// Hash-based embedding service for offline/MVP use
/// Provides lightweight semantic search without external API dependencies
pub struct HashEmbeddingService {
    dimension: usize,
}

impl HashEmbeddingService {
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }

    pub fn embed(&self, text: &str) -> Vec<f32> {
        let text_lower = text.to_lowercase();
        let words: Vec<&str> = text_lower.split_whitespace().collect();
        let mut vector = vec![0.0f32; self.dimension];

        for word in words {
            let hash = self.hash_word(word);
            let idx = (hash % self.dimension as u64) as usize;
            vector[idx] += 1.0;
        }

        let mag = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        if mag > 0.0 {
            for v in &mut vector {
                *v /= mag;
            }
        }

        vector
    }

    fn hash_word(&self, word: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        word.hash(&mut hasher);
        hasher.finish()
    }

    pub fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }
        a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
    }
}

impl Default for HashEmbeddingService {
    fn default() -> Self {
        Self::new(256)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredEmbedding {
    entity_type: String,
    entity_id: String,
    name: String,
    snippet: String,
    vector: Vec<f32>,
}

pub struct VectorStore {
    embeddings: Vec<StoredEmbedding>,
    embedding_service: HashEmbeddingService,
    use_api: bool,
}

impl VectorStore {
    pub fn new() -> Self {
        Self {
            embeddings: Vec::new(),
            embedding_service: HashEmbeddingService::default(),
            use_api: false,
        }
    }

    pub fn with_api(embedding_engine: Option<&EmbeddingEngine>) -> Self {
        Self {
            embeddings: Vec::new(),
            embedding_service: HashEmbeddingService::default(),
            use_api: embedding_engine.is_some(),
        }
    }

    pub fn add_entity(&mut self, entity_type: String, entity_id: String, name: String, snippet: String, text: &str) {
        let vector = self.embedding_service.embed(text);
        self.embeddings.push(StoredEmbedding {
            entity_type,
            entity_id,
            name,
            snippet,
            vector,
        });
    }

    pub fn search(&self, query: &str, top_k: usize) -> Vec<SemanticSearchResult> {
        let query_vector = self.embedding_service.embed(query);

        let mut results: Vec<SemanticSearchResult> = self
            .embeddings
            .iter()
            .map(|e| {
                let sim = self.embedding_service.cosine_similarity(&query_vector, &e.vector);
                SemanticSearchResult {
                    entity_type: e.entity_type.clone(),
                    entity_id: e.entity_id.clone(),
                    name: e.name.clone(),
                    similarity: sim as f64,
                    snippet: e.snippet.clone(),
                }
            })
            .collect();

        results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
        results.truncate(top_k);
        results
    }

    pub fn len(&self) -> usize {
        self.embeddings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.embeddings.is_empty()
    }

    pub fn clear(&mut self) {
        self.embeddings.clear();
    }

    pub fn embed_text(&self, text: &str) -> Vec<f32> {
        self.embedding_service.embed(text)
    }

    pub fn search_with_vector(&self, query_vector: &[f32], top_k: usize) -> Vec<SemanticSearchResult> {
        let mut results: Vec<SemanticSearchResult> = self
            .embeddings
            .iter()
            .map(|e| {
                let sim = self.embedding_service.cosine_similarity(query_vector, &e.vector);
                SemanticSearchResult {
                    entity_type: e.entity_type.clone(),
                    entity_id: e.entity_id.clone(),
                    name: e.name.clone(),
                    similarity: sim as f64,
                    snippet: e.snippet.clone(),
                }
            })
            .collect();

        results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
        results.truncate(top_k);
        results
    }
}

impl Default for VectorStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Persistent vector store backed by SQLite
pub struct PersistentVectorStore<'a> {
    conn: &'a Connection,
    embedding_service: HashEmbeddingService,
}

impl<'a> PersistentVectorStore<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self {
            conn,
            embedding_service: HashEmbeddingService::default(),
        }
    }

    pub fn init_tables(&self) -> rusqlite::Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS entity_embeddings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                entity_type TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                name TEXT NOT NULL,
                snippet TEXT,
                vector BLOB NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(entity_type, entity_id)
            )",
            [],
        )?;
        Ok(())
    }

    pub fn upsert_embedding(
        &self,
        entity_type: &str,
        entity_id: &str,
        name: &str,
        snippet: &str,
        vector: &[f32],
    ) -> rusqlite::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let vector_bytes: Vec<u8> = vector
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();

        self.conn.execute(
            "INSERT INTO entity_embeddings (entity_type, entity_id, name, snippet, vector, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(entity_type, entity_id) DO UPDATE SET
                name = excluded.name,
                snippet = excluded.snippet,
                vector = excluded.vector,
                updated_at = excluded.updated_at",
            params![entity_type, entity_id, name, snippet, vector_bytes, now],
        )?;
        Ok(())
    }

    pub fn search(&self, query: &str, top_k: usize) -> rusqlite::Result<Vec<SemanticSearchResult>> {
        let query_vector = self.embedding_service.embed(query);
        let mut stmt = self.conn.prepare(
            "SELECT entity_type, entity_id, name, snippet, vector FROM entity_embeddings",
        )?;

        let rows = stmt.query_map([], |row| {
            let entity_type: String = row.get(0)?;
            let entity_id: String = row.get(1)?;
            let name: String = row.get(2)?;
            let snippet: Option<String> = row.get(3)?;
            let vector_bytes: Vec<u8> = row.get(4)?;

            let vector: Vec<f32> = vector_bytes
                .chunks_exact(4)
                .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect();

            Ok((entity_type, entity_id, name, snippet.unwrap_or_default(), vector))
        })?;

        let mut results: Vec<SemanticSearchResult> = rows
            .filter_map(|r| r.ok())
            .map(|(entity_type, entity_id, name, snippet, vector)| {
                let sim = self.embedding_service.cosine_similarity(&query_vector, &vector);
                SemanticSearchResult {
                    entity_type,
                    entity_id,
                    name,
                    similarity: sim as f64,
                    snippet,
                }
            })
            .collect();

        results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
        results.truncate(top_k);
        Ok(results)
    }

    pub fn get_embedding(&self, entity_type: &str, entity_id: &str) -> rusqlite::Result<Option<Vec<f32>>> {
        let mut stmt = self.conn.prepare(
            "SELECT vector FROM entity_embeddings WHERE entity_type = ?1 AND entity_id = ?2",
        )?;
        let mut rows = stmt.query(params![entity_type, entity_id])?;

        match rows.next()? {
            Some(row) => {
                let vector_bytes: Vec<u8> = row.get(0)?;
                let vector: Vec<f32> = vector_bytes
                    .chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect();
                Ok(Some(vector))
            }
            None => Ok(None),
        }
    }

    pub fn delete_embedding(&self, entity_type: &str, entity_id: &str) -> rusqlite::Result<bool> {
        let affected = self.conn.execute(
            "DELETE FROM entity_embeddings WHERE entity_type = ?1 AND entity_id = ?2",
            params![entity_type, entity_id],
        )?;
        Ok(affected > 0)
    }

    pub fn count(&self) -> rusqlite::Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM entity_embeddings",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    pub fn generate_all_embeddings(&self, conn: &Connection) -> rusqlite::Result<()> {
        self.generate_skill_embeddings(conn)?;
        self.generate_experience_embeddings(conn)?;
        self.generate_project_embeddings(conn)?;
        self.generate_company_embeddings(conn)?;
        self.generate_story_embeddings(conn)?;
        Ok(())
    }

    fn generate_skill_embeddings(&self, conn: &Connection) -> rusqlite::Result<()> {
        let mut stmt = conn.prepare("SELECT id, name, category, level FROM skills")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;

        for row in rows {
            let (id, name, category, level) = row?;
            let text = format!("{} {} {}", name, category.as_deref().unwrap_or_default(), level);
            let snippet = format!("{} ({}) - {}", name, level, category.as_deref().unwrap_or_default());
            let vector = self.embedding_service.embed(&text);
            self.upsert_embedding("skill", &id, &name, &snippet, &vector)?;
        }
        Ok(())
    }

    fn generate_experience_embeddings(&self, conn: &Connection) -> rusqlite::Result<()> {
        let mut stmt = conn.prepare("SELECT id, company, role, description FROM experiences")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;

        for row in rows {
            let (id, company, role, description) = row?;
            let text = format!("{} {} {}", company, role, description.as_deref().unwrap_or_default());
            let snippet = format!("{} at {} - {}", role, company, description.as_deref().unwrap_or_default());
            let vector = self.embedding_service.embed(&text);
            self.upsert_embedding("experience", &id, &role, &snippet, &vector)?;
        }
        Ok(())
    }

    fn generate_project_embeddings(&self, conn: &Connection) -> rusqlite::Result<()> {
        let mut stmt = conn.prepare("SELECT id, name, description, keywords FROM projects")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;

        for row in rows {
            let (id, name, description, keywords) = row?;
            let text = format!("{} {} {}", name, description.as_deref().unwrap_or_default(), keywords.as_deref().unwrap_or_default());
            let snippet = format!("{} - {}", name, description.as_deref().unwrap_or_default());
            let vector = self.embedding_service.embed(&text);
            self.upsert_embedding("project", &id, &name, &snippet, &vector)?;
        }
        Ok(())
    }

    fn generate_company_embeddings(&self, conn: &Connection) -> rusqlite::Result<()> {
        let mut stmt = conn.prepare("SELECT id, name, industry, description, tech_stack FROM companies")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })?;

        for row in rows {
            let (id, name, industry, description, tech_stack) = row?;
            let text = format!("{} {} {} {}", name, industry.as_deref().unwrap_or_default(), description.as_deref().unwrap_or_default(), tech_stack.as_deref().unwrap_or_default());
            let snippet = format!("{} - {}", name, description.as_deref().unwrap_or_default());
            let vector = self.embedding_service.embed(&text);
            self.upsert_embedding("company", &id, &name, &snippet, &vector)?;
        }
        Ok(())
    }

    fn generate_story_embeddings(&self, conn: &Connection) -> rusqlite::Result<()> {
        let mut stmt = conn.prepare("SELECT id, title, situation, task, action, result FROM star_stories")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?;

        for row in rows {
            let (id, title, situation, task, action, result) = row?;
            let display_name = title.clone().unwrap_or_else(|| situation.clone());
            let text = format!("{} {} {} {} {}", title.unwrap_or_default(), situation, task, action, result);
            let vector = self.embedding_service.embed(&text);
            self.upsert_embedding("star_story", &id, &display_name, &situation, &vector)?;
        }
        Ok(())
    }
}

pub struct EmbeddingEngine {
    api_key: String,
    base_url: String,
    model: String,
}

impl EmbeddingEngine {
    pub fn new(config: &Config) -> Self {
        let model = config.embedding_model.clone().unwrap_or_else(|| {
            // Use the same model format as the LLM but for embeddings
            // text-embedding-3-small is the default for OpenAI-compatible APIs
            "text-embedding-3-small".to_string()
        });

        Self {
            api_key: config.openai_api_key.clone(),
            base_url: config.openai_base_url.trim_end_matches("/v1").to_string(),
            model,
        }
    }

    pub async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f64>>, Box<dyn std::error::Error + Send + Sync>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/v1/embeddings", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": self.model,
                "input": texts
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Embedding API error {}: {}", status, body).into());
        }

        #[derive(Deserialize)]
        struct EmbedResponse {
            data: Vec<EmbeddingData>,
        }
        #[derive(Deserialize)]
        struct EmbeddingData {
            embedding: Vec<f64>,
        }

        let emb_response: EmbedResponse = response.json().await?;
        Ok(emb_response.data.into_iter().map(|d| d.embedding).collect())
    }

    pub async fn embed_single(&self, text: &str) -> Result<Vec<f64>, Box<dyn std::error::Error + Send + Sync>> {
        let mut results = self.embed(vec![text.to_string()]).await?;
        results.pop().ok_or_else(|| "No embedding returned".into())
    }
}

pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let mag_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }
    dot / (mag_a * mag_b)
}

pub struct SemanticSearcher<'a> {
    embeddings: &'a HashMap<String, Vec<f64>>,
    entity_texts: &'a HashMap<String, (String, String, String)>, // id -> (type, name, snippet)
}

impl<'a> SemanticSearcher<'a> {
    pub fn new(
        embeddings: &'a HashMap<String, Vec<f64>>,
        entity_texts: &'a HashMap<String, (String, String, String)>,
    ) -> Self {
        Self { embeddings, entity_texts }
    }

    pub fn search(&self, query_embedding: &[f64], top_k: usize) -> Vec<SemanticSearchResult> {
        let mut results: Vec<SemanticSearchResult> = self
            .embeddings
            .iter()
            .filter_map(|(id, emb)| {
                let text_info = self.entity_texts.get(id)?;
                let sim = cosine_similarity(query_embedding, emb);
                Some(SemanticSearchResult {
                    entity_type: text_info.0.clone(),
                    entity_id: id.clone(),
                    name: text_info.1.clone(),
                    similarity: sim,
                    snippet: text_info.2.clone(),
                })
            })
            .collect();

        results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
        results.truncate(top_k);
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.0001);

        let c = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &c)).abs() < 0.0001);

        let d = vec![0.707, 0.707, 0.0];
        let sim = cosine_similarity(&a, &d);
        assert!(sim > 0.7 && sim < 0.71);
    }

    #[test]
    fn test_semantic_searcher() {
        let mut embeddings = HashMap::new();
        embeddings.insert("s1".into(), vec![1.0, 0.0, 0.0]);
        embeddings.insert("s2".into(), vec![0.0, 1.0, 0.0]);
        embeddings.insert("s3".into(), vec![0.707, 0.707, 0.0]);

        let mut texts = HashMap::new();
        texts.insert("s1".into(), ("skill".into(), "Kubernetes".into(), "K8s".into()));
        texts.insert("s2".into(), ("skill".into(), "Rust".into(), "Systems".into()));
        texts.insert("s3".into(), ("skill".into(), "Microservices".into(), "Distributed".into()));

        let searcher = SemanticSearcher::new(&embeddings, &texts);
        let results = searcher.search(&vec![1.0, 0.0, 0.0], 2);

        assert_eq!(results[0].entity_id, "s1");
        assert!(results[0].similarity > 0.99);
    }

    #[test]
    fn test_hash_embedding_service() {
        let service = HashEmbeddingService::new(128);
        
        let v1 = service.embed("kubernetes container orchestration");
        let v2 = service.embed("docker containers docker-compose");
        let v3 = service.embed("python programming language");
        
        assert_eq!(v1.len(), 128);
        assert_eq!(v2.len(), 128);
        assert_eq!(v3.len(), 128);
        
        let mag = v1.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((mag - 1.0).abs() < 0.0001, "Vector should be normalized");
        
        let sim_same = service.cosine_similarity(&v1, &v1);
        assert!((sim_same - 1.0).abs() < 0.0001, "Same text should have similarity 1.0");
        
        let sim_related = service.cosine_similarity(&v1, &v2);
        let sim_unrelated = service.cosine_similarity(&v1, &v3);
        assert!(sim_same > sim_related, "Same text should have highest similarity");
        assert!(sim_same > sim_unrelated, "Same text should beat unrelated text");
    }

    #[test]
    fn test_vector_store_search() {
        let mut store = VectorStore::new();
        
        store.add_entity(
            "skill".to_string(),
            "s1".to_string(),
            "Kubernetes".to_string(),
            "Container orchestration".to_string(),
            "kubernetes container docker",
        );
        store.add_entity(
            "skill".to_string(),
            "s2".to_string(),
            "Rust".to_string(),
            "Systems programming".to_string(),
            "rust programming memory safety",
        );
        store.add_entity(
            "skill".to_string(),
            "s3".to_string(),
            "Go".to_string(),
            "Backend language".to_string(),
            "go golang backend server",
        );
        
        let results = store.search("docker containers", 2);
        assert!(!results.is_empty());
        assert_eq!(results[0].entity_id, "s1");
        assert!(results[0].similarity > 0.3);
    }

    #[test]
    fn test_vector_store_normalized() {
        let service = HashEmbeddingService::new(64);
        let v = service.embed("test text with words");
        
        let mag = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((mag - 1.0).abs() < 0.001, "Vector should be normalized, got mag {}", mag);
    }
}