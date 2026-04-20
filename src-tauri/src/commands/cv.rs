use crate::state::AppState;
use crate::types::CvPreview;
use ghostai_pilot::knowledge::cv_parser;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn parse_cv(state: State<'_, AppState>, file_path: String) -> Result<CvPreview, String> {
    let path = std::path::Path::new(&file_path);
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();
    let text = cv_parser::extract_text(path).map_err(|e| format!("Failed to extract text from CV: {}", e))?;
    let text_length = text.len();
    let parsed_config = state.config.clone();
    let parsed = cv_parser::parse_cv_with_llm(&parsed_config, &text).await.map_err(|e| format!("Failed to parse CV with AI: {}", e))?;
    Ok(CvPreview { parsed, file_name, text_length })
}

#[tauri::command]
pub fn confirm_cv_import(state: State<'_, AppState>, parsed: cv_parser::ParsedCv) -> Result<cv_parser::CvImportResult, String> {
    let graph_provider_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
    let provider = graph_provider_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let result = cv_parser::import_parsed_cv(&graph, &parsed).map_err(|e| format!("Failed to import CV data: {}", e))?;
    Ok(result)
}

#[tauri::command]
pub async fn open_cv_dialog(app_handle: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let file_path = app_handle.dialog().file()
        .add_filter("CV Files", &["pdf", "txt", "md"])
        .set_title("Select your CV / Resume")
        .blocking_pick_file()
        .map(|p| match p {
            tauri_plugin_dialog::FilePath::Path(pathbuf) => pathbuf.to_string_lossy().to_string(),
            tauri_plugin_dialog::FilePath::Url(url) => url.to_string(),
        });
    Ok(file_path)
}
