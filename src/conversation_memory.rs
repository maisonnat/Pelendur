use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// A single conversation memory entry loaded from Engram
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub session_id: Option<String>,
    pub created_at: Option<String>,
}

/// Cross-session conversation memory backed by Engram.
///
/// Each interview gets its own Engram session. Q&A turns are saved as
/// observations. On subsequent sessions, relevant past context is loaded
/// via Engram's hybrid search (FTS5 + vector) and injected into the LLM
/// prompt as a system message.
pub struct ConversationMemory {
    engram_url: String,
    project: String,
    session_id: Option<String>,   // current Engram session UUID
    max_context_messages: usize,  // how many past memories to inject
}

impl ConversationMemory {
    /// Create a new conversation memory manager.
    ///
    /// - `engram_url`: base URL of the Engram server (e.g. "http://localhost:7437")
    /// - `project`: Engram project namespace (e.g. "pelendur")
    /// - `max_context_messages`: max number of past memories to load into context
    pub fn new(engram_url: impl Into<String>, project: impl Into<String>,
               max_context_messages: usize) -> Self {
        Self {
            engram_url: engram_url.into(),
            project: project.into(),
            session_id: None,
            max_context_messages,
        }
    }

    /// Start a new interview session in Engram.
    ///
    /// Creates an Engram session and stores its UUID so subsequent
    /// `save_turn` calls are linked to this session.
    pub async fn start_session(&mut self, title: &str) -> Result<()> {
        let url = format!("{}/sessions", self.engram_url);
        let payload = serde_json::json!({
            "project": self.project,
        });

        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .context("Failed to create Engram session")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Engram session creation failed ({}): {}", status, body);
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .context("Failed to parse Engram session response")?;

        let sid = data["session_id"]
            .as_str()
            .context("Engram response missing session_id")?
            .to_string();

        self.session_id = Some(sid.clone());

        // Also save the session title as an observation
        self.save_observation(&format!("📋 Interview: {}", title),
                              &format!("Interview session started at project '{}'. Title: {}", self.project, title))
            .await?;

        info!("Conversation memory session started: {} — {}", sid, title);
        Ok(())
    }

    /// Save a Q&A turn to the current Engram session.
    pub async fn save_turn(&self, question: &str, answer: &str) -> Result<()> {
        let title = truncate(&format!("Q: {}", question), 120);
        let content = format!(
            "Pregunta del entrevistador:\n{}\n\nRespuesta del candidato:\n{}",
            question, answer
        );
        self.save_observation(&title, &content).await
    }

    /// Save an arbitrary observation to the current Engram session.
    async fn save_observation(&self, title: &str, content: &str) -> Result<()> {
        let session_id = self.session_id.as_ref()
            .context("No active Engram session — call start_session() first")?;

        let url = format!("{}/observations", self.engram_url);
        let payload = serde_json::json!({
            "session_id": session_id,
            "title": title,
            "content": content,
            "scope": "project",
            "type": "manual",
        });

        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .context("Failed to save observation to Engram")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            warn!("Failed to save observation ({}): {}", status, body);
        }

        Ok(())
    }

    /// Search Engram for past observations relevant to `query`.
    ///
    /// Uses Engram's hybrid search (FTS5 + vector embeddings) to find
    /// semantically related conversations from previous sessions.
    pub async fn search(&self, query: &str) -> Result<Vec<MemoryEntry>> {
        let url = format!("{}/observations?query={}", self.engram_url, urlencoding(query));

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .send()
            .await
            .context("Failed to query Engram observations")?;

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        #[derive(Deserialize)]
        struct RawObservation {
            id: i64,
            #[serde(default)]
            title: String,
            #[serde(default)]
            content: String,
            #[serde(default)]
            session_id: Option<String>,
            #[serde(default)]
            created_at: Option<String>,
        }

        let observations: Vec<RawObservation> = resp
            .json()
            .await
            .unwrap_or_default();

        let mut entries: Vec<MemoryEntry> = observations
            .into_iter()
            .map(|o| MemoryEntry {
                id: o.id,
                title: o.title,
                content: o.content,
                session_id: o.session_id,
                created_at: o.created_at,
            })
            .collect();

        // Sort by id descending (newest first) and limit
        entries.sort_by(|a, b| b.id.cmp(&a.id));
        entries.truncate(self.max_context_messages);

        Ok(entries)
    }

    /// Build a system-prompt block from past session memory relevant to
    /// the current transcription.
    ///
    /// Returns `None` when no relevant context is found, so the caller can
    /// skip injecting an empty block.
    pub async fn build_memory_context(&self, current_transcription: &str)
        -> Result<Option<String>>
    {
        let entries = self.search(current_transcription).await
            .unwrap_or_default();

        if entries.is_empty() {
            return Ok(None);
        }

        let mut lines = Vec::new();
        lines.push("━━━ CONTEXTO DE ENTREVISTAS ANTERIORES ━━━".to_string());
        lines.push("Las siguientes son preguntas y respuestas de sesiones prevas.".to_string());
        lines.push("Úsalas para dar respuestas consistentes y basadas en experiencia real del candidato.".to_string());
        lines.push(String::new());

        for entry in &entries {
            let timestamp = entry.created_at.as_deref().unwrap_or("desconocido");
            lines.push(format!("── [{}] ({})", entry.title, timestamp));
            // Only include a snippet of the content to avoid flooding context
            let snippet = truncate(&entry.content, 400);
            lines.push(snippet);
            lines.push(String::new());
        }

        lines.push("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".to_string());

        Ok(Some(lines.join("\n")))
    }

    /// End the current session — saves a summary and closes it.
    pub async fn end_session(&self, summary: &str) -> Result<()> {
        if let Some(sid) = &self.session_id {
            self.save_observation(
                &truncate(&format!("📋 Session summary: {}", summary), 120),
                summary,
            ).await?;
            info!("Conversation memory session ended: {}", sid);
        }
        Ok(())
    }

    /// Current session ID, if active.
    pub fn current_session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }
}

/// Simple URL-encoding for query strings (avoids pulling in `url` crate).
fn urlencoding(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b' ' => result.push_str("%20"),
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

/// Truncate a string to `max` chars, appending "…" if cut.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long() {
        assert_eq!(truncate("hello world", 5), "hello…");
    }

    #[test]
    fn test_urlencoding_spaces() {
        assert_eq!(urlencoding("hello world"), "hello%20world");
    }

    #[test]
    fn test_urlencoding_special() {
        assert_eq!(urlencoding("a/b?c"), "a%2Fb%3Fc");
    }
}
