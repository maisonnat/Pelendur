use crate::state::AppState;
use crate::types::*;
use tauri::State;

// ─── Skills ─────────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_skills(state: State<'_, AppState>) -> Result<Vec<SkillRecord>, String> {
    let gp_lock = state.graph_provider.blocking_lock();
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let skills = graph.list_skills().map_err(|e| format!("Failed to list skills: {}", e))?;
    Ok(skills.into_iter().map(SkillRecord::from).collect())
}

#[tauri::command]
pub fn create_skill(state: State<'_, AppState>, data: SkillData) -> Result<SkillRecord, String> {
    let gp_lock = state.graph_provider.blocking_lock();
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let entity = graph.create_skill(&data.name, data.category.as_deref(), &data.level, data.years, None)
        .map_err(|e| format!("Failed to create skill: {}", e))?;
    Ok(entity.into())
}

#[tauri::command]
pub fn update_skill(state: State<'_, AppState>, data: SkillData) -> Result<SkillRecord, String> {
    let id = data.id.as_ref().ok_or_else(|| "Skill ID required for update".to_string())?;
    let gp_lock = state.graph_provider.blocking_lock();
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let existing = graph.get_skill(id).map_err(|e| e.to_string())?.ok_or_else(|| format!("Skill {} not found", id))?;
    let updated = ghostai_pilot::knowledge::graph::SkillEntity {
        id: existing.id, name: data.name, category: data.category, level: data.level,
        years: data.years, source: existing.source, created_at: existing.created_at, updated_at: existing.updated_at,
    };
    graph.update_skill(&updated).map_err(|e| format!("Failed to update skill: {}", e))?;
    Ok(updated.into())
}

#[tauri::command]
pub fn delete_skill(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    let gp_lock = state.graph_provider.blocking_lock();
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    graph.delete_skill(&id).map_err(|e| format!("Failed to delete skill: {}", e))
}

// ─── Experiences ────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_experiences_with_skills(state: State<'_, AppState>) -> Result<Vec<ExperienceWithSkills>, String> {
    let gp_lock = state.graph_provider.blocking_lock();
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let experiences = graph.list_experiences().map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for exp in experiences {
        let edges = graph.get_edges_for_entity(&exp.id, ghostai_pilot::knowledge::graph::EntityType::Experience).map_err(|e| e.to_string())?;
        let mut skill_names = Vec::new();
        for edge in &edges {
            let connected_id = if edge.source_id == exp.id { &edge.target_id } else { &edge.source_id };
            if let Ok(Some(skill)) = graph.get_skill(connected_id) { skill_names.push(skill.name); }
        }
        result.push(ExperienceWithSkills {
            id: exp.id, company: exp.company, role: exp.role,
            start_date: exp.start_date, end_date: exp.end_date,
            description: exp.description, highlights: exp.highlights, skills: skill_names,
        });
    }
    Ok(result)
}

#[tauri::command]
pub fn create_experience(state: State<'_, AppState>, data: ExperienceData) -> Result<ExperienceWithSkills, String> {
    let gp_lock = state.graph_provider.blocking_lock();
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let entity = graph.create_experience(&data.company, &data.role, &data.start_date, data.end_date.as_deref(), data.description.as_deref(), data.highlights.as_deref())
        .map_err(|e| format!("Failed to create experience: {}", e))?;
    let mut skill_names = Vec::new();
    if let Some(skill_ids) = &data.skill_ids {
        for skill_id in skill_ids {
            let _ = graph.add_edge(&entity.id, ghostai_pilot::knowledge::graph::EntityType::Experience, skill_id, ghostai_pilot::knowledge::graph::EntityType::Skill, "used", 1.0);
            if let Ok(Some(skill)) = graph.get_skill(skill_id) { skill_names.push(skill.name); }
        }
    }
    Ok(ExperienceWithSkills {
        id: entity.id, company: entity.company, role: entity.role,
        start_date: entity.start_date, end_date: entity.end_date,
        description: entity.description, highlights: entity.highlights, skills: skill_names,
    })
}

#[tauri::command]
pub fn update_experience(state: State<'_, AppState>, data: ExperienceData) -> Result<ExperienceWithSkills, String> {
    let id = data.id.as_ref().ok_or_else(|| "Experience ID required for update".to_string())?;
    let gp_lock = state.graph_provider.blocking_lock();
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let existing = graph.get_experience(id).map_err(|e| e.to_string())?.ok_or_else(|| format!("Experience {} not found", id))?;
    let updated = ghostai_pilot::knowledge::graph::ExperienceEntity {
        id: existing.id, company: data.company, role: data.role,
        start_date: data.start_date, end_date: data.end_date,
        description: data.description, highlights: data.highlights,
    };
    graph.update_experience(&updated).map_err(|e| format!("Failed to update experience: {}", e))?;
    if let Some(skill_ids) = &data.skill_ids {
        let old_edges = graph.get_edges_for_entity(id, ghostai_pilot::knowledge::graph::EntityType::Experience).map_err(|e| e.to_string())?;
        for edge in old_edges { if edge.target_type == "skill" || edge.source_type == "skill" { let _ = graph.remove_edge(&edge.id); } }
        for skill_id in skill_ids { let _ = graph.add_edge(id, ghostai_pilot::knowledge::graph::EntityType::Experience, skill_id, ghostai_pilot::knowledge::graph::EntityType::Skill, "used", 1.0); }
    }
    let mut skill_names = Vec::new();
    let edges = graph.get_edges_for_entity(id, ghostai_pilot::knowledge::graph::EntityType::Experience).map_err(|e| e.to_string())?;
    for edge in &edges {
        let connected_id = if edge.source_id == *id { &edge.target_id } else { &edge.source_id };
        if let Ok(Some(skill)) = graph.get_skill(connected_id) { skill_names.push(skill.name); }
    }
    Ok(ExperienceWithSkills {
        id: updated.id, company: updated.company, role: updated.role,
        start_date: updated.start_date, end_date: updated.end_date,
        description: updated.description, highlights: updated.highlights, skills: skill_names,
    })
}

#[tauri::command]
pub fn delete_experience(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    let gp_lock = state.graph_provider.blocking_lock();
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let edges = graph.get_edges_for_entity(&id, ghostai_pilot::knowledge::graph::EntityType::Experience).map_err(|e| e.to_string())?;
    for edge in edges { let _ = graph.remove_edge(&edge.id); }
    graph.delete_experience(&id).map_err(|e| format!("Failed to delete experience: {}", e))
}

// ─── STAR Stories ───────────────────────────────────────────────────────

#[tauri::command]
pub fn create_star_story(state: State<'_, AppState>, data: StarStoryData) -> Result<StarStoryRecord, String> {
    let gp_lock = state.graph_provider.blocking_lock();
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let entity = graph.create_star_story(data.title.as_deref(), &data.situation, &data.task, &data.action, &data.result, data.tags.as_deref(), data.difficulty.as_deref(), data.stakes.as_deref())
        .map_err(|e| format!("Failed to create STAR story: {}", e))?;
    Ok(entity.into())
}

#[tauri::command]
pub fn update_star_story(state: State<'_, AppState>, data: StarStoryData) -> Result<StarStoryRecord, String> {
    let id = data.id.as_ref().ok_or_else(|| "Story ID required for update".to_string())?;
    let gp_lock = state.graph_provider.blocking_lock();
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let existing = graph.get_star_story(id).map_err(|e| e.to_string())?.ok_or_else(|| format!("STAR story {} not found", id))?;
    let updated = ghostai_pilot::knowledge::graph::StarStoryEntity {
        id: existing.id, title: data.title, situation: data.situation, task: data.task,
        action: data.action, result: data.result, tags: data.tags, difficulty: data.difficulty,
        stakes: data.stakes, usage_count: existing.usage_count, created_at: existing.created_at, updated_at: existing.updated_at,
    };
    graph.update_star_story(&updated).map_err(|e| format!("Failed to update STAR story: {}", e))?;
    Ok(updated.into())
}

#[tauri::command]
pub fn delete_star_story(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    let gp_lock = state.graph_provider.blocking_lock();
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    graph.delete_star_story(&id).map_err(|e| format!("Failed to delete STAR story: {}", e))
}

#[tauri::command]
pub fn get_star_stories(state: State<'_, AppState>) -> Result<Vec<StarStoryRecord>, String> {
    let gp_lock = state.graph_provider.blocking_lock();
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let stories = graph.list_star_stories().map_err(|e| format!("Failed to list STAR stories: {}", e))?;
    Ok(stories.into_iter().map(StarStoryRecord::from).collect())
}

#[tauri::command]
pub async fn coach_star_story(state: State<'_, AppState>, story_id: Option<String>, question: String) -> Result<String, String> {
    let config = state.config.clone();
    let story_context = if let Some(sid) = story_id {
        let result = {
            let gp_lock = state.graph_provider.blocking_lock();
            let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
            let graph = provider.graph();
            let story = graph.get_star_story(&sid).map_err(|e| e.to_string())?;
            story.map(|s| format!("Here is the user's STAR story:\nTitle: {}\nSituation: {}\nTask: {}\nAction: {}\nResult: {}\nTags: {}\nDifficulty: {}\nStakes: {}",
                s.title.as_deref().unwrap_or("Untitled"), s.situation, s.task, s.action, s.result,
                s.tags.as_deref().unwrap_or("none"), s.difficulty.as_deref().unwrap_or("unspecified"), s.stakes.as_deref().unwrap_or("unspecified")))
        };
        result
    } else { None };

    let system_msg = ghostai_pilot::llm::ChatMessage { role: "system".to_string(), content: "You are an expert interview coach specializing in the STAR method (Situation, Task, Action, Result). Help the user improve their STAR stories for behavioral interviews. Provide specific, actionable suggestions. Be concise (2-4 sentences max per suggestion). Focus on: specificity, quantifiable results, personal impact, and clarity of narrative.".to_string() };
    let mut messages = vec![system_msg];
    if let Some(ctx) = story_context { messages.push(ghostai_pilot::llm::ChatMessage { role: "system".to_string(), content: ctx }); }
    messages.push(ghostai_pilot::llm::ChatMessage { role: "user".to_string(), content: question });

    let response = ghostai_pilot::llm::generate_response_with_options(&config, &messages, 800).await.map_err(|e| format!("LLM coaching error: {}", e))?;
    Ok(response)
}

// ─── Edges ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn add_edge(state: State<'_, AppState>, source_id: String, source_type: String, target_id: String, target_type: String, relation: String, weight: f64) -> Result<EdgeRecord, String> {
    let gp_lock = state.graph_provider.blocking_lock();
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let src_t = parse_entity_type(&source_type)?;
    let tgt_t = parse_entity_type(&target_type)?;
    let edge = graph.add_edge(&source_id, src_t, &target_id, tgt_t, &relation, weight).map_err(|e| format!("Failed to add edge: {}", e))?;
    Ok(edge.into())
}

#[tauri::command]
pub fn remove_edge(state: State<'_, AppState>, edge_id: String) -> Result<bool, String> {
    let gp_lock = state.graph_provider.blocking_lock();
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    graph.remove_edge(&edge_id).map_err(|e| format!("Failed to remove edge: {}", e))
}

#[tauri::command]
pub fn list_edges_for_entity(state: State<'_, AppState>, entity_id: String, entity_type: String) -> Result<Vec<EdgeRecord>, String> {
    let gp_lock = state.graph_provider.blocking_lock();
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let et = parse_entity_type(&entity_type)?;
    let edges = graph.get_edges_for_entity(&entity_id, et).map_err(|e| format!("Failed to list edges: {}", e))?;
    Ok(edges.into_iter().map(EdgeRecord::from).collect())
}

// ─── Full Graph Data ────────────────────────────────────────────────────

#[tauri::command]
pub fn get_graph_data(state: State<'_, AppState>) -> Result<GraphData, String> {
    let gp_lock = state.graph_provider.blocking_lock();
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();

    let skills = graph.list_skills().map_err(|e| e.to_string())?.into_iter().map(SkillRecord::from).collect();
    let experiences_raw = graph.list_experiences().map_err(|e| e.to_string())?;
    let mut experiences = Vec::new();
    for exp in experiences_raw {
        let edges = graph.get_edges_for_entity(&exp.id, ghostai_pilot::knowledge::graph::EntityType::Experience).unwrap_or_default();
        let mut skill_names = Vec::new();
        for edge in &edges {
            if edge.source_type == "skill" { if let Ok(Some(s)) = graph.get_skill(&edge.source_id) { skill_names.push(s.name); } }
            else if edge.target_type == "skill" { if let Ok(Some(s)) = graph.get_skill(&edge.target_id) { skill_names.push(s.name); } }
        }
        experiences.push(ExperienceWithSkills {
            id: exp.id, company: exp.company, role: exp.role, start_date: exp.start_date, end_date: exp.end_date,
            description: exp.description, highlights: exp.highlights, skills: skill_names,
        });
    }
    let star_stories = graph.list_star_stories().map_err(|e| e.to_string())?.into_iter().map(StarStoryRecord::from).collect();
    let companies = graph.list_companies().map_err(|e| e.to_string())?.into_iter().map(CompanyRecord::from).collect();
    let edges = graph.list_all_edges().map_err(|e| e.to_string())?.into_iter().map(EdgeRecord::from).collect();

    Ok(GraphData { skills, experiences, star_stories, companies, edges })
}