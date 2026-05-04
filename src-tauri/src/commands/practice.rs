use crate::state::AppState;
use ghostai_pilot::knowledge::company::CompanyLoader;
use tauri::State;

#[tauri::command]
pub async fn generate_practice_questions(
    state: State<'_, AppState>,
    mode: String,
    company_name: Option<String>,
) -> Result<Vec<serde_json::Value>, String> {
    let config = &state.config;

    let (profile_summary, company_context) = {
        let km = state.knowledge_manager.lock().map_err(|e| e.to_string())?;

        // Build profile summary
        let profile = if let Some(p) = &km.personal_profile {
            let all_skills: Vec<String> = p.skills.dominados.iter()
                .chain(p.skills.intermedios.iter())
                .map(|s| s.nombre.clone()).collect();
            format!(
                "Name: {}\nTitle: {}\nSkills: {}\nExperience: {} years",
                p.nombre, p.rol_actual,
                all_skills.join(", "),
                p.experiencia
            )
        } else {
            "Unknown professional".to_string()
        };

        // Load company research context if company_name is provided
        let ctx = if let Some(ref company) = company_name {
            let loader = CompanyLoader::new("knowledge");
            match loader.get_interview_context(company) {
                Ok(research) => research,
                Err(_) => format!("No research found for {}", company),
            }
        } else {
            String::new()
        };

        (profile, ctx)
    };

    // Inject company context into question generation
    let full_profile = if !company_context.is_empty() {
        format!("{}\n\n{}", profile_summary, company_context)
    } else {
        profile_summary
    };

    let questions = ghostai_pilot::knowledge::practice::PracticeEngine::generate_questions(
        &mode,
        &full_profile,
        company_name.as_deref(),
        config,
    ).await.map_err(|e| e.to_string())?;

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
    let feedback = ghostai_pilot::knowledge::practice::PracticeEngine::analyze_answer(
        &question, &answer, &mode, config,
    ).await.map_err(|e| e.to_string())?;

    serde_json::to_value(feedback).map_err(|e| e.to_string())
}
