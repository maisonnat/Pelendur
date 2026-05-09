use crate::state::AppState;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, State, WebviewWindow, WebviewWindowBuilder, WebviewUrl};

// ── Readiness / System Health ──────────────────────────────────

#[derive(Serialize, Clone)]
pub struct ReadinessReport {
    pub stt: String,         // "ready" | "warming" | "error"
    pub llm: String,         // "connected" | "local" | "offline"
    pub kg: String,          // "ready" | "error"
    pub audio: String,       // "idle" | "capturing" | "error"
    pub overall: String,     // "ready" | "limited" | "critical"
    pub stt_model: String,   // e.g. "whisper-tiny-multilingual"
    pub llm_model: String,   // e.g. "qwen3:4b-instruct"
    pub latency_ms: u64,     // last STT latency
    pub uptime_seconds: u64,
    pub transcription_count: u64,
    pub errors: Vec<String>,
}

#[tauri::command]
pub fn get_readiness(state: State<'_, AppState>) -> ReadinessReport {
    #[cfg(feature = "testing")]
    {
        let metrics = state.test_metrics.lock().unwrap();
        let last_latency = metrics.stt_latency_ms.last().map(|l| l.1).unwrap_or(0);
        let uptime = metrics.uptime_seconds;
        let count = metrics.transcription_count;
        let errors = metrics.errors.clone();

        let audio_state = {
            let streams = state.active_streams.lock().unwrap();
            if !streams.is_empty() { "capturing" } else { "idle" }
        };

        let _latency_status = if last_latency < 2000 { "fast" } else if last_latency < 5000 { "ok" } else { "slow" };

        let overall = match (audio_state, &metrics.errors.len()) {
            ("capturing", 0) => "ready",
            ("idle", 0) => "limited",
            _ => "critical",
        };

        return ReadinessReport {
            stt: "ready".to_string(),
            llm: "connected".to_string(),
            kg: "ready".to_string(),
            audio: audio_state.to_string(),
            overall: overall.to_string(),
            stt_model: "whisper-tiny-multilingual".to_string(),
            llm_model: state.config.openai_model.clone(),
            latency_ms: last_latency,
            uptime_seconds: uptime,
            transcription_count: count,
            errors,
        };
    }

    #[cfg(not(feature = "testing"))]
    ReadinessReport {
        stt: "ready".to_string(),
        llm: "connected".to_string(),
        kg: "ready".to_string(),
        audio: "idle".to_string(),
        overall: "ready".to_string(),
        stt_model: "whisper-tiny-multilingual".to_string(),
        llm_model: state.config.openai_model.clone(),
        latency_ms: 0,
        uptime_seconds: 0,
        transcription_count: 0,
        errors: vec![],
    }
}

#[derive(Serialize)]
pub struct SystemStatus {
    pub stt: String,
    pub llm: String,
    pub kg: String,
}

#[tauri::command]
pub async fn get_system_status() -> Result<SystemStatus, String> {
    Ok(SystemStatus {
        stt: "ready".to_string(),
        llm: "ready".to_string(),
        kg: "ready".to_string(),
    })
}

#[tauri::command]
pub fn set_lock_state(window: WebviewWindow, state: State<'_, AppState>, locked: bool) -> Result<(), String> {
    let mut is_locked = state.is_locked.lock().map_err(|e| e.to_string())?;
    *is_locked = locked;
    let _ = window.set_ignore_cursor_events(locked);
    let _ = window.emit("lock-state-changed", locked);
    Ok(())
}

#[tauri::command]
pub fn set_minimal_mode(window: WebviewWindow, state: State<'_, AppState>, minimal: bool) -> Result<(), String> {
    let mut is_minimal = state.is_minimal.lock().map_err(|e| e.to_string())?;
    *is_minimal = minimal;

    if minimal {
        // Shrink to a tiny floating icon in the bottom-right corner
        window.set_size(PhysicalSize::new(64, 64)).map_err(|e| e.to_string())?;
        // Position near bottom-right (adjust for taskbar)
        window.set_position(PhysicalPosition::new(1840, 1000)).map_err(|e| e.to_string())?;
    } else {
        // Restore to full size at original position
        window.set_size(PhysicalSize::new(800, 400)).map_err(|e| e.to_string())?;
        window.set_position(PhysicalPosition::new(560, 50)).map_err(|e| e.to_string())?;
    }

    let _ = window.emit("minimal-mode-changed", minimal);
    Ok(())
}

#[tauri::command]
pub fn clear_feed(state: State<'_, AppState>) -> Result<(), String> {
    let mut conversation = state.conversation.lock().map_err(|e| e.to_string())?;
    if !conversation.is_empty() {
        conversation.truncate(1);
    }
    Ok(())
}

#[tauri::command]
pub fn regenerate(app_handle: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let conversation = state.conversation.lock().map_err(|e| e.to_string())?.clone();
    let config = state.config.clone();
    if conversation.len() < 2 { return Err("No data".to_string()); }

    let app = app_handle.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            use ghostai_pilot::llm;
            use crate::state::SuggestionPayload;
            if let Ok(response) = llm::generate_response(&config, &conversation).await {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.emit("suggestion-update", SuggestionPayload { text: response });
                }
            }
        });
    });
    Ok(())
}

/// Close the application entirely.
#[tauri::command]
pub fn close_app(app_handle: AppHandle) -> Result<(), String> {
    println!("  Closing Pelendur...");
    app_handle.exit(0);
    Ok(())
}

#[tauri::command]
pub fn open_profile_window(app_handle: AppHandle) -> Result<(), String> {
    if let Some(existing) = app_handle.get_webview_window("profile") {
        let _ = existing.set_focus();
        let _ = existing.show();
        return Ok(());
    }

    let url = WebviewUrl::App("ui-profile/index.html".into());
    WebviewWindowBuilder::new(&app_handle, "profile", url)
        .title("Pelendur - Profile Management")
        .inner_size(900.0, 700.0)
        .center()
        .decorations(true)
        .resizable(true)
        .content_protected(true)
        .build()
        .map_err(|e| format!("Failed to create profile window: {}", e))?;

    Ok(())
}
