//! Cross-session conversation memory using Engram (the Crab Engram).
//!
//! Stores interview Q&A turns as observations in Engram sessions.
//! Provides context loading of past interview sessions for LLM injection.
//!
//! # Architecture
//!
//! Each interview session creates a corresponding Engram session.
//! Q&A turns are stored as observations tagged with `session_id` and
//! a `topic_key` of `pelendur/interview/{company}` for semantic search.
//! When starting a new interview, past context is loaded via `/search`
//! and injected into the LLM system prompt.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::info;

/// A single conversation turn stored in Engram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryTurn {
    /// The question asked (user message)
    pub question: String,
    /// The LLM suggestion (assistant message)
    pub answer: String,
    /// Timestamp of the turn
    pub recorded_at: String,
}

/// A session summary from past interviews.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PastSession {
    pub company: String,
    pub session_id: String,
    pub turns: usize,
    pub started_at: String,
    pub summary: Option<String>,
}

/// Observation as returned by the Engram search API.
#[derive(Debug, Clone, Deserialize)]
struct EngramObservation {
    #[allow(dead_code)]
    id: i64,
    title: String,
    content: String,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    obs_type: String,
    session_id: String,
    #[allow(dead_code)]
    project: String,
    #[allow(dead_code)]
    topic_key: Option<String>,
    created_at: String,
}

/// Create session response from Engram.
#[derive(Debug, Deserialize)]
struct CreateSessionResponse {
    session_id: String,
}

/// Search response from Engram — returns an array of observations.
type SearchResponse = Vec<EngramObservation>;

/// Cross-session conversation memory backed by Engram.
#[derive(Debug, Clone)]
pub struct ConversationMemory {
    /// Base URL of the Engram server (e.g., http://localhost:7437)
    engram_base_url: String,
    /// Project namespace in Engram (e.g., "pelendur")
    project: String,
}

impl ConversationMemory {
    /// Create a new ConversationMemory instance.
    pub fn new(engram_base_url: String, project: Option<String>) -> Self {
        Self {
            engram_base_url,
            project: project.unwrap_or_else(|| "pelendur".to_string()),
        }
    }

    /// Create a new Engram session for an interview.
    pub async fn create_session(&self) -> Result<String> {
        let url = format!("{}/sessions", self.engram_base_url);
        let client = reqwest::Client::new();

        let body = serde_json::json!({
            "project": self.project,
        });

        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to create Engram session")?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Engram create session error {}: {}", status, text);
        }

        let result: CreateSessionResponse = resp
            .json()
            .await
            .context("Failed to parse Engram create session response")?;

        info!("Created Engram session: {}", result.session_id);
        Ok(result.session_id)
    }

    /// Store a single Q&A turn in the given Engram session.
    pub async fn store_turn(
        &self,
        session_id: &str,
        company: &str,
        question: &str,
        answer: &str,
    ) -> Result<()> {
        let url = format!("{}/observations", self.engram_base_url);
        let client = reqwest::Client::new();

        // Truncate long content for the title
        let question_trunc = truncate(question, 60);
        let topic_key = format!("pelendur/interview/{company}");

        // Store the question
        let q_body = serde_json::json!({
            "title": format!("Q: {question_trunc}"),
            "content": format!("Interview question from {company} interview.\n\nQ: {question}\nA: {answer}"),
            "type": "manual",
            "scope": "project",
            "session_id": session_id,
            "project": self.project,
            "topic_key": &topic_key,
        });

        let resp = client
            .post(&url)
            .json(&q_body)
            .send()
            .await
            .context("Failed to store turn in Engram")?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Engram store turn error {}: {}", status, text);
        }

        info!("Stored interview turn in Engram session {session_id}");
        Ok(())
    }

    /// End an Engram session with a summary.
    pub async fn end_session(&self, session_id: &str, summary: &str) -> Result<()> {
        let url = format!("{}/sessions/{session_id}/end", self.engram_base_url);
        let client = reqwest::Client::new();

        let body = serde_json::json!({
            "summary": summary,
        });

        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to end Engram session")?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Engram end session error {}: {}", status, text);
        }

        info!("Ended Engram session {session_id}");
        Ok(())
    }

    /// Load past interview context for a given company.
    /// Returns a markdown-formatted string ready for LLM system prompt injection.
    pub async fn load_past_context(&self, company: &str, limit: usize) -> Result<String> {
        let topic_key = format!("pelendur/interview/{company}");

        let url = format!("{}/search", self.engram_base_url);
        let client = reqwest::Client::new();

        let body = serde_json::json!({
            "query": topic_key,
            "limit": limit,
            "project": self.project,
        });

        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to search Engram for past context")?;

        if !resp.status().is_success() {
            // Not a fatal error — just return empty context
            tracing::warn!(
                "Engram search returned {}: returning empty context",
                resp.status()
            );
            return Ok(String::new());
        }

        let observations: SearchResponse = resp
            .json()
            .await
            .context("Failed to parse Engram search response")?;

        if observations.is_empty() {
            return Ok(String::new());
        }

        // Group observations by session_id for coherent context
        let mut sessions: Vec<(String, Vec<EngramObservation>)> = Vec::new();
        for obs in &observations {
            let idx = sessions.iter().position(|(sid, _)| sid == &obs.session_id);
            if let Some(pos) = idx {
                sessions[pos].1.push(obs.clone());
            } else {
                sessions.push((obs.session_id.clone(), vec![obs.clone()]));
            }
        }

        let mut context = String::from("\n\n### PAST INTERVIEW SESSIONS\n\n");
        for (session_id, turns) in &sessions {
            context.push_str(&format!("**Session**: {session_id}\n"));
            for turn in turns {
                context.push_str(&format!("- {}\n", turn.title));
                context.push_str(&format!("  {}\n\n", truncate(&turn.content, 150)));
            }
        }

        info!(
            "Loaded {} past observations for company {company}",
            observations.len()
        );
        Ok(context)
    }

    /// Quick check if Engram is reachable.
    pub async fn health_check(&self) -> bool {
        let url = format!("{}/health", self.engram_base_url);
        match reqwest::get(&url).await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }
}

/// Truncate a string to `max_len` characters, appending "…" if truncated.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5), "hello…");
        assert_eq!(truncate("", 5), "");
    }

    #[test]
    fn test_new_memory() {
        let mem = ConversationMemory::new("http://localhost:7437".into(), Some("pelendur".into()));
        assert_eq!(mem.engram_base_url, "http://localhost:7437");
        assert_eq!(mem.project, "pelendur");
    }

    #[test]
    fn test_new_memory_default_project() {
        let mem = ConversationMemory::new("http://localhost:7437".into(), None);
        assert_eq!(mem.project, "pelendur");
    }
}
