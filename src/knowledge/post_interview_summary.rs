//! F4: Post-interview Summary Generator
//!
//! After each interview session, generates an automatic structured summary:
//! 1. Questions answered
//! 2. Responses used (copied/accepted by the user)
//! 3. AI-suggested responses NOT used
//! 4. Most frequent topics/themes
//! 5. Confidence score per topic
//! 6. Areas without prepared answers (gaps)
//!
//! Output: `knowledge/interviews/Interview_Summary_YYYY-MM-DD_HHmm.md`
//!
//! Connects with E4 (Engram-backed hybrid search) to improve future responses.

use crate::config::Config;
use crate::conversation_memory::{self, MemoryEntry};
use crate::llm::{self, ChatMessage};
use anyhow::{Context, Result};
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info, warn};

// ── Public Types ────────────────────────────────────────────────────────────

/// Tracks whether an AI-suggested response was used (accepted/copied) by the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerUsage {
    /// The interview question (transcription)
    pub question: String,
    /// The AI-suggested response
    pub suggested_answer: String,
    /// Whether the user copied or accepted this answer
    pub was_used: bool,
    /// Optional timestamp of when the answer was shown
    pub timestamp: Option<String>,
}

/// A single analyzed topic extracted from the session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicAnalysis {
    /// The topic name (e.g., "Rust async patterns", "SQL optimization")
    pub topic: String,
    /// How many times this topic appeared
    pub frequency: u32,
    /// AI-assessed confidence (0.0–1.0) for answers on this topic
    pub confidence: f32,
    /// Whether the topic has a prepared STAR story / knowledge entry
    pub has_prepared_answer: bool,
}

/// The full post-interview analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterviewSummary {
    /// Date/time of the interview
    pub date: String,
    /// Duration estimate (number of Q&A turns)
    pub total_turns: usize,
    /// All questions asked during the session
    pub questions_answered: Vec<AnswerUsage>,
    /// AI suggestions that were accepted/copied
    pub used_responses: Vec<AnswerUsage>,
    /// AI suggestions shown but NOT accepted
    pub unused_responses: Vec<AnswerUsage>,
    /// Topics identified + confidence scores
    pub topic_analysis: Vec<TopicAnalysis>,
    /// Topics that lack a prepared answer (gaps)
    pub gaps: Vec<String>,
    /// Overall session confidence (0.0–1.0)
    pub overall_confidence: f32,
    /// Raw markdown report generated
    pub markdown_report: String,
}

// ── Session Usage Tracker ───────────────────────────────────────────────────

/// In-memory tracker for answer usage during an active interview session.
///
/// The HUD or main loop calls `record_suggestion()` whenever an AI response
/// is shown, and `mark_as_used()` when the user copies/accepts it.
pub struct AnswerUsageTracker {
    entries: Vec<AnswerUsage>,
}

impl AnswerUsageTracker {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Record that an AI suggestion was shown to the user.
    pub fn record_suggestion(&mut self, question: &str, answer: &str) {
        self.entries.push(AnswerUsage {
            question: question.to_string(),
            suggested_answer: answer.to_string(),
            was_used: false,
            timestamp: Some(Local::now().format("%H:%M:%S").to_string()),
        });
    }

    /// Mark the most recent suggestion as used (accepted/copied by the user).
    pub fn mark_last_as_used(&mut self) {
        if let Some(last) = self.entries.last_mut() {
            last.was_used = true;
        }
    }

    /// Get all entries.
    pub fn entries(&self) -> &[AnswerUsage] {
        &self.entries
    }

    /// Partition entries into used and unused.
    pub fn partition(&self) -> (Vec<AnswerUsage>, Vec<AnswerUsage>) {
        let mut used = Vec::new();
        let mut unused = Vec::new();
        for entry in &self.entries {
            if entry.was_used {
                used.push(entry.clone());
            } else {
                unused.push(entry.clone());
            }
        }
        (used, unused)
    }

    /// Number of total suggestions tracked.
    pub fn total(&self) -> usize {
        self.entries.len()
    }

    /// Number of used (accepted) suggestions.
    pub fn used_count(&self) -> usize {
        self.entries.iter().filter(|e| e.was_used).count()
    }

    /// Usage rate (0.0–1.0). Returns 0.0 if no suggestions tracked.
    pub fn usage_rate(&self) -> f32 {
        if self.entries.is_empty() {
            return 0.0;
        }
        self.used_count() as f32 / self.entries.len() as f32
    }
}

// ── Summary Generator ───────────────────────────────────────────────────────

/// Generate a post-interview summary from session data.
///
/// # Arguments
/// * `config` - App configuration (for LLM access)
/// * `memory` - Conversation memory (for Engram session access)
/// * `usage_tracker` - Tracked answer usage from the session
/// * `session_title` - Title of the interview session
///
/// Returns the generated `InterviewSummary` and writes the markdown report to disk.
pub async fn generate_summary(
    config: &Config,
    memory: &conversation_memory::ConversationMemory,
    usage_tracker: &AnswerUsageTracker,
    session_title: &str,
) -> Result<InterviewSummary> {
    let now = Local::now();
    let date_str = now.format("%Y-%m-%d %H:%M").to_string();
    let file_timestamp = now.format("%Y-%m-%d_%H%M").to_string();

    // 1. Fetch all observations from the current Engram session
    let session_observations = fetch_session_observations(memory).await?;

    // 2. Build Q&A pairs from the observations
    let _questions: Vec<String> = session_observations
        .iter()
        .filter(|o| o.title.starts_with("Q:") || o.title.starts_with("Pregunta"))
        .map(|o| o.content.clone())
        .collect();

    // 3. Partition used vs unused answers
    let (used, unused) = usage_tracker.partition();

    // 4. Use LLM to analyze the conversation and extract topics + confidence
    let topic_analysis = analyze_topics(config, &session_observations, usage_tracker).await?;

    // 5. Identify gaps (areas without prepared answers)
    let gaps = identify_gaps(&topic_analysis);

    // 6. Calculate overall confidence
    let overall_confidence = if topic_analysis.is_empty() {
        0.0
    } else {
        topic_analysis.iter().map(|t| t.confidence).sum::<f32>() / topic_analysis.len() as f32
    };

    // 7. Build the markdown report
    let markdown_report = build_report(
        &date_str,
        session_title,
        &session_observations,
        &used,
        &unused,
        &topic_analysis,
        &gaps,
        overall_confidence,
        usage_tracker.usage_rate(),
    );

    // 8. Write to disk
    let report_path = write_report_to_disk(&file_timestamp, &markdown_report)?;

    // 9. Save analysis back to Engram for E4 future improvement
    save_summary_to_engram(memory, &markdown_report, session_title).await?;

    info!("Post-interview summary saved to: {}", report_path.display());

    // Also save to knowledge/interviews via knowledge manager
    let knowledge_dir = PathBuf::from("knowledge/interviews");
    if !knowledge_dir.exists() {
        fs::create_dir_all(&knowledge_dir).context("Failed to create knowledge/interviews dir")?;
    }
    let kpath = knowledge_dir.join(format!("Interview_Summary_{}.md", file_timestamp));
    fs::write(&kpath, &markdown_report).context("Failed to write summary to knowledge/")?;

    Ok(InterviewSummary {
        date: date_str,
        total_turns: session_observations.len(),
        questions_answered: usage_tracker.entries().to_vec(),
        used_responses: used,
        unused_responses: unused,
        topic_analysis,
        gaps,
        overall_confidence,
        markdown_report,
    })
}

// ── Internal Helpers ────────────────────────────────────────────────────────

/// Fetch all observations from the active Engram session.
async fn fetch_session_observations(
    memory: &conversation_memory::ConversationMemory,
) -> Result<Vec<MemoryEntry>> {
    // Use a broad query to fetch recent observations from this session
    // We search for "Interview" which captures session metadata and Q&A turns
    let mut entries = memory.search("Interview").await.unwrap_or_default();

    // Also fetch Q&A observations
    let qa_entries = memory.search("Pregunta").await.unwrap_or_default();
    entries.extend(qa_entries);

    // Also fetch user transcription-based entries
    let user_entries = memory.search("entrevistador").await.unwrap_or_default();
    entries.extend(user_entries);

    // Deduplicate by ID
    entries.sort_by(|a, b| b.id.cmp(&a.id));
    entries.dedup_by(|a, b| a.id == b.id);

    debug!("Fetched {} observations from Engram session", entries.len());
    Ok(entries)
}

/// Use the LLM to analyze topics, confidence, and gaps from the session.
async fn analyze_topics(
    config: &Config,
    observations: &[MemoryEntry],
    usage_tracker: &AnswerUsageTracker,
) -> Result<Vec<TopicAnalysis>> {
    // Build a compact summary of the session for the LLM
    let mut session_text = String::new();
    for obs in observations {
        let snippet = if obs.content.len() > 300 {
            format!("{}…", &obs.content[..300])
        } else {
            obs.content.clone()
        };
        session_text.push_str(&format!("[{}] {}\n", obs.title, snippet));
        session_text.push('\n');
    }

    // Add usage info
    let usage_info = if usage_tracker.entries().is_empty() {
        "No AI suggestions recorded yet.".to_string()
    } else {
        let used_count = usage_tracker.used_count();
        let total = usage_tracker.total();
        format!(
            "AI suggestions: {}/{} accepted (usage rate: {:.0}%).",
            used_count,
            total,
            usage_tracker.usage_rate() * 100.0
        )
    };

    let prompt = format!(
        r#"Eres un analista de entrevistas especializado en evaluar el desempeño de un candidato.

A continuación tienes el registro de una sesión de entrevista (preguntas y respuestas):

{}

{}

Analiza la sesión y extrae los temas/tópicos principales que aparecieron en las preguntas.
Para cada tema, asigna:
- Un nombre descriptivo (máximo 60 caracteres)
- Una frecuencia (número de veces que apareció)
- Un score de confianza (0.0 a 1.0) basado en qué tan bien respondió el candidato
- Si el candidato tenía una respuesta preparada (basada en conocimiento o STAR stories)

Responde SOLO con un JSON válido en este formato exacto, sin markdown ni explicaciones:
{{"topics": [
  {{"topic": "nombre del tema", "frequency": 3, "confidence": 0.85, "has_prepared_answer": true}},
  ...
]}}
"#,
        session_text, usage_info
    );

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: "Eres un analista de entrevistas preciso y objetivo. Respondes exclusivamente con JSON.".to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: prompt,
        },
    ];

    // Use a generous token limit for analysis
    let response = llm::generate_response_with_options(config, &messages, 2000)
        .await
        .context("LLM topic analysis failed")?;

    // Parse JSON response
    let cleaned = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    #[derive(Deserialize)]
    struct AnalysisResponse {
        topics: Vec<TopicAnalysis>,
    }

    match serde_json::from_str::<AnalysisResponse>(cleaned) {
        Ok(parsed) => {
            debug!("LLM analyzed {} topics", parsed.topics.len());
            Ok(parsed.topics)
        }
        Err(e) => {
            warn!(
                "Failed to parse LLM topic analysis: {} — raw: {}",
                e,
                cleaned.chars().take(200).collect::<String>()
            );
            // Return a basic fallback analysis
            Ok(Vec::new())
        }
    }
}

/// Identify topics that lack a prepared answer.
fn identify_gaps(topics: &[TopicAnalysis]) -> Vec<String> {
    topics
        .iter()
        .filter(|t| !t.has_prepared_answer && t.confidence < 0.6)
        .map(|t| t.topic.clone())
        .collect()
}

/// Build the markdown report content.
fn build_report(
    date: &str,
    title: &str,
    observations: &[MemoryEntry],
    used: &[AnswerUsage],
    unused: &[AnswerUsage],
    topics: &[TopicAnalysis],
    gaps: &[String],
    overall_confidence: f32,
    usage_rate: f32,
) -> String {
    let mut report = String::new();

    // Header
    report.push_str(&format!("# 📋 Post-Interview Summary\n\n"));
    report.push_str(&format!("**Session:** {}\n", title));
    report.push_str(&format!("**Date:** {}\n", date));
    report.push_str(&format!("**Total Q&A turns:** {}\n\n", observations.len()));

    // Metrics dashboard
    report.push_str("## 📊 Performance Dashboard\n\n");
    report.push_str("| Metric | Value |\n");
    report.push_str("|--------|-------|\n");
    report.push_str(&format!(
        "| Overall Confidence | {:.0}% |\n",
        overall_confidence * 100.0
    ));
    report.push_str(&format!(
        "| Response Usage Rate | {:.0}% |\n",
        usage_rate * 100.0
    ));
    report.push_str(&format!("| Topics Identified | {} |\n", topics.len()));
    report.push_str(&format!("| Knowledge Gaps | {} |\n", gaps.len()));
    report.push_str(&format!("| Used Answers | {} |\n", used.len()));
    report.push_str(&format!("| Unused Suggestions | {} |\n\n", unused.len()));

    // Topic analysis
    if !topics.is_empty() {
        report.push_str("## 🎯 Topic Analysis & Confidence Scores\n\n");
        report.push_str("| Topic | Frequency | Confidence | Prepared? |\n");
        report.push_str("|-------|-----------|------------|-----------|\n");
        for t in topics {
            let icon = if t.has_prepared_answer { "✅" } else { "⚠️" };
            let confidence_bar = confidence_bar(t.confidence);
            report.push_str(&format!(
                "| {} | {}× | {} {:.0}% | {} |\n",
                t.topic, t.frequency, confidence_bar, t.confidence * 100.0, icon
            ));
        }
        report.push('\n');
    }

    // Used responses
    if !used.is_empty() {
        report.push_str("## ✅ Answers Used (Accepted/Copied)\n\n");
        for (i, ans) in used.iter().enumerate() {
            report.push_str(&format!("### {}.\n", i + 1));
            report.push_str(&format!("**Q:** {}\n\n", ans.question));
            report.push_str(&format!("**A:** {}\n\n", ans.suggested_answer));
            if let Some(ts) = &ans.timestamp {
                report.push_str(&format!("_{}_\n\n", ts));
            }
        }
    }

    // Unused suggestions
    if !unused.is_empty() {
        report.push_str("## ❌ AI Suggestions Not Used\n\n");
        for (i, ans) in unused.iter().enumerate() {
            report.push_str(&format!("### {}.\n", i + 1));
            report.push_str(&format!("**Q:** {}\n\n", ans.question));
            report.push_str(&format!("**Suggested:** {}\n\n", ans.suggested_answer));
        }
    }

    // Knowledge gaps
    if !gaps.is_empty() {
        report.push_str("## 🔴 Knowledge Gaps (Areas Without Prepared Answers)\n\n");
        report.push_str("These topics need STAR story preparation:\n\n");
        for gap in gaps {
            report.push_str(&format!("- {}\n", gap));
        }
        report.push('\n');
    }

    // Full conversation log
    if !observations.is_empty() {
        report.push_str("## 📜 Full Conversation Log\n\n");
        for obs in observations {
            let snippet = if obs.content.len() > 500 {
                format!("{}…", &obs.content[..500])
            } else {
                obs.content.clone()
            };
            report.push_str(&format!("**{}:**\n{}\n\n", obs.title, snippet));
        }
    }

    // Footer
    report.push_str("---\n");
    report.push_str(&format!(
        "_Generated by Pelendur F4 on {}_\n",
        Local::now().format("%Y-%m-%d %H:%M:%S")
    ));

    report
}

/// Save the summary back to Engram as an observation for E4 to consume.
async fn save_summary_to_engram(
    memory: &conversation_memory::ConversationMemory,
    report: &str,
    session_title: &str,
) -> Result<()> {
    let summary_title = format!("📊 Post-Interview Summary: {}", session_title);
    let summary_content = report.chars().take(2000).collect::<String>();

    // Save as an observation via the session's end_session mechanism
    // end_session already saves a summary observation — we add more detail
    if let Some(session_id) = memory.current_session_id() {
        let engram_url = format!(
            "{}/observations",
            std::env::var("ENGRAM_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:7437".to_string())
        );

        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "session_id": session_id,
            "title": summary_title,
            "content": summary_content,
            "scope": "project",
            "type": "analysis",
        });

        if let Ok(resp) = client.post(&engram_url).json(&payload).send().await {
            if resp.status().is_success() {
                info!("Summary saved to Engram for E4 consumption");
            } else {
                warn!("Failed to save summary to Engram: {}", resp.status());
            }
        } else {
            warn!("Failed to connect to Engram for summary save");
        }
    }

    Ok(())
}

/// Write the markdown report to disk in the workspace output directory.
fn write_report_to_disk(timestamp: &str, report: &str) -> Result<PathBuf> {
    // Try multiple possible output locations
    let paths = vec![
        // Standard knowledge/interviews dir (in project root)
        format!("knowledge/interviews/Interview_Summary_{}.md", timestamp),
        // Current working directory fallback
        format!("Interview_Summary_{}.md", timestamp),
    ];

    for path_str in &paths {
        if let Some(parent) = PathBuf::from(path_str).parent() {
            if !parent.as_os_str().is_empty() {
                let _ = fs::create_dir_all(parent);
            }
        }
        if let Err(e) = fs::write(path_str, report) {
            debug!("Failed to write summary to {}: {}", path_str, e);
        } else {
            return Ok(PathBuf::from(path_str));
        }
    }

    anyhow::bail!("Failed to write summary to any output location")
}

/// Generate a visual confidence bar (e.g., "█████░░░░░").
fn confidence_bar(confidence: f32) -> String {
    let filled = (confidence * 10.0).round() as usize;
    let filled = filled.clamp(0, 10);
    let empty = 10 - filled;
    format!(
        "{}",
        "█".repeat(filled) + &"░".repeat(empty)
    )
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_answer_usage_tracker_empty() {
        let tracker = AnswerUsageTracker::new();
        assert_eq!(tracker.total(), 0);
        assert_eq!(tracker.usage_rate(), 0.0);
        assert!(tracker.partition().0.is_empty());
        assert!(tracker.partition().1.is_empty());
    }

    #[test]
    fn test_answer_usage_tracker_record() {
        let mut tracker = AnswerUsageTracker::new();
        tracker.record_suggestion("What is Rust?", "Rust is a systems language.");
        assert_eq!(tracker.total(), 1);
        assert_eq!(tracker.used_count(), 0);
        assert_eq!(tracker.usage_rate(), 0.0);
    }

    #[test]
    fn test_answer_usage_tracker_mark_used() {
        let mut tracker = AnswerUsageTracker::new();
        tracker.record_suggestion("Q1", "A1");
        tracker.record_suggestion("Q2", "A2");
        tracker.mark_last_as_used();

        assert_eq!(tracker.total(), 2);
        assert!(tracker.entries()[1].was_used);
        assert!(!tracker.entries()[0].was_used);

        let (used, unused) = tracker.partition();
        assert_eq!(used.len(), 1);
        assert_eq!(unused.len(), 1);
    }

    #[test]
    fn test_answer_usage_tracker_usage_rate() {
        let mut tracker = AnswerUsageTracker::new();
        tracker.record_suggestion("Q1", "A1");
        tracker.record_suggestion("Q2", "A2");
        tracker.record_suggestion("Q3", "A3");
        tracker.mark_last_as_used();

        assert!((tracker.usage_rate() - 0.333).abs() < 0.01);
    }

    #[test]
    fn test_confidence_bar_full() {
        let bar = confidence_bar(1.0);
        assert_eq!(bar, "██████████");
    }

    #[test]
    fn test_confidence_bar_half() {
        let bar = confidence_bar(0.5);
        assert_eq!(bar, "█████░░░░░");
    }

    #[test]
    fn test_confidence_bar_empty() {
        let bar = confidence_bar(0.0);
        assert_eq!(bar, "░░░░░░░░░░");
    }

    #[test]
    fn test_confidence_bar_clamp() {
        let bar = confidence_bar(1.5);
        assert_eq!(bar, "██████████");
    }

    #[test]
    fn test_identify_gaps_empty() {
        let gaps = identify_gaps(&[]);
        assert!(gaps.is_empty());
    }

    #[test]
    fn test_identify_gaps_no_gaps() {
        let topics = vec![TopicAnalysis {
            topic: "Rust".to_string(),
            frequency: 3,
            confidence: 0.9,
            has_prepared_answer: true,
        }];
        let gaps = identify_gaps(&topics);
        assert!(gaps.is_empty());
    }

    #[test]
    fn test_identify_gaps_actual_gaps() {
        let topics = vec![
            TopicAnalysis {
                topic: "Kubernetes".to_string(),
                frequency: 2,
                confidence: 0.3,
                has_prepared_answer: false,
            },
            TopicAnalysis {
                topic: "Rust async".to_string(),
                frequency: 3,
                confidence: 0.45,
                has_prepared_answer: false,
            },
            TopicAnalysis {
                topic: "Python".to_string(),
                frequency: 1,
                confidence: 0.9,
                has_prepared_answer: true,
            },
        ];
        let gaps = identify_gaps(&topics);
        assert_eq!(gaps.len(), 2);
        assert!(gaps.contains(&"Kubernetes".to_string()));
        assert!(gaps.contains(&"Rust async".to_string()));
    }

    #[test]
    fn test_build_report_basic() {
        let report = build_report(
            "2025-01-01 10:00",
            "Test Session",
            &[],
            &[],
            &[],
            &[],
            &[],
            0.5,
            0.0,
        );
        assert!(report.contains("Post-Interview Summary"));
        assert!(report.contains("Test Session"));
        assert!(report.contains("Performance Dashboard"));
    }
}
