use crate::state::{AppState, HudState, TestMetrics};
use ghostai_pilot::stt;
use std::sync::Arc;
use std::sync::Mutex;
use tauri::State;

fn uptime() -> u64 {
    static START: std::sync::LazyLock<std::time::Instant> =
        std::sync::LazyLock::new(std::time::Instant::now);
    START.elapsed().as_secs()
}

fn get_hud_state_inner(state: &AppState) -> HudState {
    let is_locked = *state.is_locked.lock().unwrap();
    let is_minimal = *state.is_minimal.lock().unwrap();
    let interview_active = state.interview_session.lock().unwrap().is_some();
    let conversation = state.conversation.lock().unwrap();
    let last_transcript = conversation
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
        .unwrap_or_default();
    let last_suggestion = conversation
        .iter()
        .rev()
        .find(|m| m.role == "assistant")
        .map(|m| m.content.clone())
        .unwrap_or_default();
    let capture_mode = state
        .test_metrics
        .lock()
        .unwrap()
        .capture_mode
        .clone();
    HudState {
        capture_mode,
        is_locked,
        is_minimal,
        interview_active,
        last_transcript,
        last_suggestion,
    }
}

#[tauri::command]
pub fn get_test_metrics(state: State<'_, AppState>) -> TestMetrics {
    let mut metrics = state.test_metrics.lock().unwrap().clone();
    metrics.uptime_seconds = uptime();
    metrics
}

#[tauri::command]
pub fn inject_test_audio(
    state: State<'_, AppState>,
    path: String,
) -> Result<String, String> {
    let wav_bytes = std::fs::read(&path).map_err(|e| format!("Failed to read WAV file: {}", e))?;
    let config = state.config.clone();
    let start = std::time::Instant::now();
    let result = stt::transcribe_local_sync(&config, &wav_bytes)
        .map_err(|e| format!("STT failed: {}", e))?;
    let latency = start.elapsed().as_millis() as u64;

    let mut metrics = state.test_metrics.lock().unwrap();
    metrics.stt_latency_ms.push((result.clone(), latency));
    metrics.transcription_count += 1;

    Ok(result)
}

#[tauri::command]
pub fn get_hud_state(state: State<'_, AppState>) -> HudState {
    get_hud_state_inner(&state)
}

#[tauri::command]
pub fn simulate_keyboard(
    state: State<'_, AppState>,
    shortcut: String,
) -> Result<(), String> {
    match shortcut.as_str() {
        "Ctrl+Alt+L" => {
            let mut is_locked = state.is_locked.lock().unwrap();
            *is_locked = !*is_locked;
            Ok(())
        }
        "Ctrl+Shift+Q" => {
            std::process::exit(0);
        }
        _ => Err(format!("Unknown shortcut: {}", shortcut)),
    }
}

#[tauri::command]
pub fn set_mode(
    state: State<'_, AppState>,
    mode: String,
) -> Result<(), String> {
    if !["system", "mic", "dual"].contains(&mode.as_str()) {
        return Err(format!("Invalid mode: {}. Use system, mic, or dual.", mode));
    }
    {
        let mut streams = state.active_streams.lock().unwrap();
        streams.clear();
    }
    {
        let mut metrics = state.test_metrics.lock().unwrap();
        metrics.capture_mode = mode;
    }
    Ok(())
}

#[tauri::command]
pub fn reset_metrics(state: State<'_, AppState>) {
    let mut metrics = state.test_metrics.lock().unwrap();
    *metrics = TestMetrics::default();
}
