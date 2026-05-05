use crate::state::AppState;
use crate::types::*;
use ghostai_pilot::knowledge::company::{CompanyLoader, CompanyResearch};
use ghostai_pilot::knowledge::graph::KnowledgeGraph;
use std::path::Path;
use tauri::State;
use tokio::task;

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
    let gp_lock = state.graph_provider.blocking_lock();
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let companies = graph.list_companies().map_err(|e| format!("Failed to list companies: {}", e))?;
    Ok(companies.into_iter().map(CompanyRecord::from).collect())
}

// ─── Create ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn create_company(state: State<'_, AppState>, data: CompanyData) -> Result<CompanyRecord, String> {
    let gp_lock = state.graph_provider.blocking_lock();
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
    let gp_lock = state.graph_provider.blocking_lock();
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
    let gp_lock = state.graph_provider.blocking_lock();
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    graph.delete_company(&id).map_err(|e| format!("Failed to delete company: {}", e))
}

// ─── Load from overview.md ───────────────────────────────────────────────

#[tauri::command]
pub fn load_company_research(state: State<'_, AppState>, company_name: String) -> Result<CompanyRecord, String> {
    let gp_lock = state.graph_provider.blocking_lock();
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
    let gp_lock = state.graph_provider.blocking_lock();
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();

    let loader = CompanyLoader::new("knowledge");
    let entities = loader.load_all_into_graph(&graph)
        .map_err(|e| format!("Failed to load company research: {}", e))?;

    Ok(entities.into_iter().map(CompanyRecord::from).collect())
}

// ─── Research Company via NotebookLM ────────────────────────────────────

/// IPC type for research status.
#[derive(serde::Serialize, Clone)]
pub struct ResearchStatusIpc {
    pub company_name: String,
    pub overview_path: String,
    pub has_notebooklm: bool,
    pub research_done: bool,
    pub message: String,
}

/// Trigger deep research for a company using NotebookLM + LLM extraction.
/// Acquires graph_provider lock briefly to validate state, then drops it before async.
#[tauri::command]
pub async fn research_company(
    state: State<'_, AppState>,
    company_name: String,
) -> Result<ResearchStatusIpc, String> {
    // Quick state check — lock and immediately drop
    let _gp_check = state.graph_provider.blocking_lock();
    _gp_check.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;

    // Create researcher and run
    let researcher = ghostai_pilot::knowledge::company_research::CompanyResearcher::default_with_path("knowledge");
    let status = researcher.research_company(&company_name, None)
        .await
        .map_err(|e| format!("Company research failed: {}", e))?;

    Ok(ResearchStatusIpc {
        company_name: status.company_name,
        overview_path: status.overview_path.to_string_lossy().to_string(),
        has_notebooklm: status.has_notebooklm,
        research_done: status.research_done,
        message: status.message,
    })
}

/// List companies that need research (stubs with no real data).
#[tauri::command]
pub async fn list_unresearched_companies(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let _gp_check = state.graph_provider.blocking_lock();
    _gp_check.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let researcher = ghostai_pilot::knowledge::company_research::CompanyResearcher::default_with_path("knowledge");
    researcher.list_unresearched_companies()
        .await
        .map_err(|e| format!("Failed to list unresearched companies: {}", e))
}

/// Research all companies that don't have real research data yet.
#[tauri::command]
pub async fn research_all_companies(state: State<'_, AppState>) -> Result<Vec<ResearchStatusIpc>, String> {
    let _gp_check = state.graph_provider.blocking_lock();
    _gp_check.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;

    let researcher = ghostai_pilot::knowledge::company_research::CompanyResearcher::default_with_path("knowledge");
    let results = researcher.research_all_missing(None)
        .await
        .map_err(|e| format!("Batch research failed: {}", e))?;

    Ok(results.into_iter().map(|s| ResearchStatusIpc {
        company_name: s.company_name,
        overview_path: s.overview_path.to_string_lossy().to_string(),
        has_notebooklm: s.has_notebooklm,
        research_done: s.research_done,
        message: s.message,
    }).collect())
}