use crate::state::{AppState, InterviewSession, SuggestionPayload, TranscriptionPayload};
use crate::types::{CompanyInfo, InterviewSessionState, InterviewSummary};
use ghostai_pilot::knowledge;
use ghostai_pilot::llm::{self, ChatMessage};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

/// Load company research from knowledge/companies/<name>/overview.md
/// If no research exists and auto_research is true, triggers NotebookLM research.
fn load_company_research(
    knowledge_base_path: &str,
    company_name: &str,
    auto_research: bool,
) -> Option<CompanyInfo> {
    let slug = company_name.to_lowercase().replace(' ', "-").replace('/', "-");
    let overview_path = Path::new(knowledge_base_path)
        .join("companies")
        .join(&slug)
        .join("overview.md");

    if !overview_path.exists() {
        if auto_research {
            // Auto-research will be triggered asynchronously from the caller
            return None;
        }
        return None;
    }

    let content = fs::read_to_string(overview_path).ok()?;
    // Skip TEMPLATE stubs
    if content.contains("{{COMPANY_NAME}}") || content.trim().len() < 100 {
        if auto_research {
            return None;
        }
    }

    let lines: Vec<&str> = content.lines().collect();
    let industry = lines
        .iter()
        .find(|l| l.contains("Industry"))
        .and_then(|l| l.split(':').nth(1))
        .map(|s| s.trim().trim_matches(',').trim().to_string());

    Some(CompanyInfo {
        name: company_name.to_string(),
        industry,
        overview: content,
    })
}

/// Build the company context string for the LLM system prompt
fn build_company_context_string(company: &str, knowledge_base_path: &str) -> String {
    let company_info = load_company_research(knowledge_base_path, company, false);
    match company_info {
        Some(info) => format!(
            r#"### CURRENT INTERVIEW CONTEXT:
Company: {name}
Industry: {industry}
Company Research:
{overview}

Use this company context to tailor your suggestions. Reference specific
company needs, tech stack, and culture in your advice.

"#,
            name = info.name,
            industry = info.industry.as_deref().unwrap_or("Unknown"),
            overview = info.overview,
        ),
        None => format!(
            r#"### CURRENT INTERVIEW CONTEXT:
Company: {company}

No detailed research available for this company. Provide general interview best practices.

"#,
            company = company,
        ),
    }
}

#[tauri::command]
pub fn start_interview(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    company_name: String,
) -> Result<CompanyInfo, String> {
    // Prevent double-starts
    {
        let session = state.interview_session.lock().map_err(|e| e.to_string())?;
        if session.is_some() {
            return Err("An interview is already in progress. End it first.".to_string());
        }
    }

    // TODO: Restore Engram session start with tokio::sync::Mutex for memory
    // Currently skipped because std::sync::MutexGuard can't cross .await in tokio::spawn

    // Load past interview context (best-effort, skip if Engram unavailable)
    let past_context = String::new();

    // Load company research
    let company_info = load_company_research("knowledge", &company_name, false);
    let company_context = build_company_context_string(&company_name, "knowledge");

    // Set interview session with Engram session_id
    {
        let mut session = state
            .interview_session
            .lock()
            .map_err(|e| e.to_string())?;
        *session = Some(InterviewSession {
            company: company_name.clone(),
            company_context: company_context.clone(),
            started_at: chrono::Utc::now(),
            turn_count: 0,
        });
    }

    emit_to_window(&app_handle, "interview-state-changed", InterviewSessionState {
        active: true,
        company: Some(company_name.clone()),
        started_at: Some(chrono::Utc::now().to_rfc3339()),
        duration_seconds: Some(0),
    });

    // Inject company context + past interview context into the conversation system prompt
    {
        let mut conversation = state.conversation.lock().map_err(|e| e.to_string())?;
        let context_block = format!("{company_context}\n{past_context}");
        if !conversation.is_empty() && conversation[0].role == "system" {
            // Append context to existing system prompt
            conversation[0].content.push_str("\n\n");
            conversation[0].content.push_str(&context_block);
        } else {
            // Create fresh system prompt with context
            let km = state.knowledge_manager.lock().map_err(|e| e.to_string())?;
            let profile_prompt = knowledge::personal::generate_system_prompt(&km);
            let full_prompt = format!("{profile_prompt}\n\n{context_block}");
            conversation.insert(0, ChatMessage {
                role: "system".to_string(),
                content: full_prompt,
            });
        }
    }

    println!("  🎬 Interview mode ACTIVE — company: {}", company_info.as_ref().map_or(&company_name, |c| &c.name));

    Ok(company_info.unwrap_or(CompanyInfo {
        name: company_name,
        industry: None,
        overview: "No research loaded.".to_string(),
    }))
}

#[tauri::command]
pub async fn end_interview(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<InterviewSummary, String> {
    let company_name;
    let started_at;
    let company_context;

    // Extract and clear session
    {
        let mut session = state
            .interview_session
            .lock()
            .map_err(|e| e.to_string())?;
        let s = session.take().ok_or("No active interview to end.".to_string())?;
        company_name = s.company;
        started_at = s.started_at;
        company_context = s.company_context;
    }

    let duration = (chrono::Utc::now() - started_at).num_seconds() as u64;

    // Get conversation history
    let (transcript_count, transcript_text) = {
        let conversation = state.conversation.lock().map_err(|e| e.to_string())?;
        let user_messages: Vec<&ChatMessage> = conversation
            .iter()
            .filter(|m| m.role == "user")
            .collect();
        let count = user_messages.len();
        let text: Vec<String> = user_messages
            .iter()
            .enumerate()
            .map(|(i, m)| format!("Q{}: {}", i + 1, m.content))
            .collect();
        (count, text.join("\n"))
    };

    let config = state.config.clone();
    let profile_prompt = {
        let km = state.knowledge_manager.lock().map_err(|e| e.to_string())?;
        knowledge::personal::generate_system_prompt(&km)
    };

    let summary_prompt = ChatMessage {
        role: "user".to_string(),
        content: format!(
            r#"Generate a comprehensive post-interview summary in Spanish.

Company: {company}
| Duration: {duration_seconds} seconds
Questions asked: {transcript_count}

Full transcript:
{transcript}

Company Context:
{company_context}

Profile Context:
{profile_context}

Please structure your response as follows:

## Summary
Brief 2-3 sentence overview of how the interview went.

## Key Strengths Demonstrated
- Point 1
- Point 2

## Areas to Improve
- Point 1
- Point 2

## Recommended STAR Stories to Highlight Next Time
- Story suggestion 1
- Story suggestion 2

## Key Terms/Topics Mentioned
- Topic 1
- Topic 2
"#,
            company = company_name,
            duration_seconds = duration,
            transcript_count = transcript_count,
            transcript = if transcript_text.is_empty() {
                "No questions were asked during this interview.".to_string()
            } else {
                transcript_text
            },
            company_context = company_context,
            profile_context = profile_prompt,
        ),
    };

    let summary_text = if transcript_count > 0 {
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are a professional interview coach. Analyze the interview transcript and provide actionable feedback.".to_string(),
            },
            summary_prompt,
        ];
        llm::generate_response_with_options(&config, &messages, 1000)
            .await
            .unwrap_or_else(|e| format!("Error generating summary: {}", e))
    } else {
        "No conversation data to summarize.".to_string()
    };

    // Save to knowledge/interviews/
    let timestamp = started_at.format("%Y-%m-%d_%H%M");
    let filename = format!("Interview_Session_{company_name}_{timestamp}.md");
    let save_path = Path::new("knowledge")
        .join("interviews")
        .join(&filename);

    if let Some(parent) = save_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let mut file = fs::File::create(&save_path).map_err(|e| e.to_string())?;
    writeln!(file, "# Interview Session Summary — {company_name}").ok();
    writeln!(file).ok();
    writeln!(file, "- **Date**: {}", started_at.format("%Y-%m-%d %H:%M")).ok();
    writeln!(file, "- **Duration**: {duration}s").ok();
    writeln!(file, "- **Transcript Count**: {transcript_count}").ok();
    writeln!(file).ok();
    writeln!(file, "---").ok();
    writeln!(file).ok();
    writeln!(file, "{summary_text}").ok();

    println!(
        "  📝 Interview summary saved to: {:?}",
        save_path.display()
    );

    // TODO: End Engram session — skipped for Send safety (std::sync::MutexGuard across .await)
    // Restore with tokio::sync::Mutex for memory field when Engram integration is prioritized

    // Emit state change to frontend
    emit_to_window(
        &app_handle,
        "interview-state-changed",
        InterviewSessionState {
            active: false,
            company: None,
            started_at: None,
            duration_seconds: Some(duration),
        },
    );

    // Parse summary into structured sections for the frontend
    let strengths = extract_section(&summary_text, "## Key Strengths Demonstrated");
    let areas = extract_section(&summary_text, "## Areas to Improve");
    let stories = extract_section(&summary_text, "## Recommended STAR Stories");

    let summary = InterviewSummary {
        company: company_name,
        duration_seconds: duration,
        transcript_count,
        summary_text,
        strengths,
        areas_to_improve: areas,
        recommended_stories: stories,
    };

    // Emit summary to frontend
    emit_to_window(&app_handle, "interview-summary", summary.clone());

    Ok(summary)
}

#[tauri::command]
pub fn get_interview_state(state: State<'_, AppState>) -> Result<InterviewSessionState, String> {
    let session = state.interview_session.lock().map_err(|e| e.to_string())?;
    match session.as_ref() {
        Some(s) => {
            let duration = (chrono::Utc::now() - s.started_at).num_seconds() as u64;
            Ok(InterviewSessionState {
                active: true,
                company: Some(s.company.clone()),
                started_at: Some(s.started_at.to_rfc3339()),
                duration_seconds: Some(duration),
            })
        }
        None => Ok(InterviewSessionState {
            active: false,
            company: None,
            started_at: None,
            duration_seconds: None,
        }),
    }
}

#[tauri::command]
pub fn list_company_dirs(state: State<'_, AppState>) -> Result<Vec<CompanyInfo>, String> {
    let companies_dir = Path::new("knowledge").join("companies");
    if !companies_dir.exists() {
        return Ok(vec![]);
    }

    let mut companies = Vec::new();
    if let Ok(entries) = fs::read_dir(&companies_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                let overview_path = entry.path().join("overview.md");
                let overview = fs::read_to_string(&overview_path).unwrap_or_default();
                let industry = overview
                    .lines()
                    .find(|l| l.contains("Industry"))
                    .and_then(|l| l.split(':').nth(1))
                    .map(|s| s.trim().trim_matches(',').trim().to_string());

                companies.push(CompanyInfo {
                    name,
                    industry,
                    overview: overview.lines().next().unwrap_or("").to_string(),
                });
            }
        }
    }
    Ok(companies)
}

/// Extract a section from markdown text by heading name
fn extract_section(text: &str, heading: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut in_section = false;

    for line in text.lines() {
        if line.starts_with(heading) {
            in_section = true;
            continue;
        }
        if in_section {
            if line.starts_with("## ") {
                break;
            }
            if line.starts_with("- ") || line.starts_with("* ") {
                let item = line
                    .trim_start_matches("- ")
                    .trim_start_matches("* ")
                    .to_string();
                if !item.is_empty() {
                    results.push(item);
                }
            }
        }
    }

    results
}

/// Emit an event to the main window, with fallback to global emit.
fn emit_to_window<T: serde::Serialize + Clone>(app: &AppHandle, event: &str, payload: T) {
    use tauri::Emitter;
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit(event, payload);
    } else {
        let _ = app.emit(event, payload);
    }
}