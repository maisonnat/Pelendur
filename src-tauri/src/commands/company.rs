use crate::state::AppState;
use crate::types::*;
use ghostai_pilot::knowledge::company::{CompanyLoader, CompanyResearch};
use ghostai_pilot::knowledge::graph::KnowledgeGraph;
use std::path::Path;
use tauri::State;

// ─── Company Data for IPC ───────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct CompanyData {
    pub id: Option<String>,
    pub name: String,
    pub industry: Option<String>,
    pub description: Option<String>,
    pub culture: Option<String>,
    pub tech_stack: Option<String>,
    pub strategic_angle: Option<String>,
}

// ─── List ────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_companies(state: State<'_, AppState>) -> Result<Vec<CompanyRecord>, String> {
    let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let companies = graph.list_companies().map_err(|e| format!("Failed to list companies: {}", e))?;
    Ok(companies.into_iter().map(CompanyRecord::from).collect())
}

// ─── Create ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn create_company(state: State<'_, AppState>, data: CompanyData) -> Result<CompanyRecord, String> {
    let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let entity = graph.create_company(
        &data.name,
        data.industry.as_deref(),
        data.description.as_deref(),
        data.culture.as_deref(),
        data.tech_stack.as_deref(),
        data.strategic_angle.as_deref(),
    ).map_err(|e| format!("Failed to create company: {}", e))?;
    Ok(entity.into())
}

// ─── Update ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn update_company(state: State<'_, AppState>, data: CompanyData) -> Result<CompanyRecord, String> {
    let id = data.id.as_ref().ok_or_else(|| "Company ID required for update".to_string())?;
    let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let existing = graph.get_company(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Company {} not found", id))?;

    let updated = ghostai_pilot::knowledge::graph::CompanyEntity {
        id: existing.id,
        name: data.name,
        industry: data.industry,
        description: data.description,
        culture: data.culture,
        tech_stack: data.tech_stack,
        strategic_angle: data.strategic_angle,
    };
    graph.update_company(&updated).map_err(|e| format!("Failed to update company: {}", e))?;
    Ok(updated.into())
}

// ─── Delete ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn delete_company(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    graph.delete_company(&id).map_err(|e| format!("Failed to delete company: {}", e))
}

// ─── Load from overview.md ───────────────────────────────────────────────

#[tauri::command]
pub fn load_company_research(state: State<'_, AppState>, company_name: String) -> Result<CompanyRecord, String> {
    let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();

    let loader = CompanyLoader::new("knowledge");
    let dir_name = company_name.to_lowercase().replace(' ', "-");
    let overview_path = Path::new("knowledge").join("companies").join(&dir_name).join("overview.md");

    if !overview_path.exists() {
        return Err(format!("No overview.md found for company '{}' at {:?}", company_name, overview_path));
    }

    let research = CompanyResearch::from_markdown(&overview_path)
        .map_err(|e| format!("Failed to parse overview.md: {}", e))?;
    let entity = loader.sync_to_graph(&research, &graph)
        .map_err(|e| format!("Failed to sync to graph: {}", e))?;

    Ok(entity.into())
}

// ─── Get company research context for interview ──────────────────────────

#[tauri::command]
pub fn get_company_research_context(state: State<'_, AppState>, company_name: String) -> Result<String, String> {
    let loader = CompanyLoader::new("knowledge");
    loader.get_interview_context(&company_name)
        .map_err(|e| format!("Failed to get research context: {}", e))
}

// ─── Refresh: reload all company overviews from disk ─────────────────────

#[tauri::command]
pub fn refresh_company_research(state: State<'_, AppState>) -> Result<Vec<CompanyRecord>, String> {
    let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();

    let loader = CompanyLoader::new("knowledge");
    let entities = loader.load_all_into_graph(&graph)
        .map_err(|e| format!("Failed to load company research: {}", e))?;

    Ok(entities.into_iter().map(CompanyRecord::from).collect())
}
