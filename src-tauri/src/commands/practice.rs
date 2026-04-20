use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn generate_practice_questions(
    state: State<'_, AppState>,
    mode: String,
    company_name: Option<String>,
) -> Result<Vec<serde_json::Value>, String> {
    let config = &state.config;
    let profile_summary = {
        let km = state.knowledge_manager.lock().map_err(|e| e.to_string())?;
        if let Some(profile) = &km.personal_profile {
            let all_skills: Vec<String> = profile.skills.dominados.iter()
                .chain(profile.skills.intermedios.iter())
                .map(|s| s.nombre.clone()).collect();
            format!("Name: {}\nTitle: {}\nSkills: {}\nExperience: {}", profile.nombre, profile.rol_actual, all_skills.join(", "), profile.experiencia)
        } else { "Unknown professional".to_string() }
    };
    let questions = ghostai_pilot::knowledge::practice::PracticeEngine::generate_questions(&mode, &profile_summary, company_name.as_deref(), config)
        .await.map_err(|e| e.to_string())?;
    questions.into_iter().map(|q| serde_json::to_value(q).map_err(|e| e.to_string())).collect()
}

#[tauri::command]
pub async fn analyze_practice_answer(
    state: State<'_, AppState>,
    question: String,
    answer: String,
    mode: String,
) -> Result<serde_json::Value, String> {
    let config = &state.config;
    let feedback = ghostai_pilot::knowledge::practice::PracticeEngine::analyze_answer(&question, &answer, &mode, config)
        .await.map_err(|e| e.to_string())?;
    serde_json::to_value(feedback).map_err(|e| e.to_string())
}
