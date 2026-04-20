use crate::state::AppState;
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow, WebviewWindowBuilder, WebviewUrl};

#[tauri::command]
pub fn set_lock_state(window: WebviewWindow, state: State<'_, AppState>, locked: bool) -> Result<(), String> {
    let mut is_locked = state.is_locked.lock().map_err(|e| e.to_string())?;
    *is_locked = locked;
    let _ = window.set_ignore_cursor_events(locked);
    let _ = window.emit("lock-state-changed", locked);
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
