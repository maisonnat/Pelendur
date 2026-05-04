use crate::state::AppState;
use crate::types::*;
use ghostai_pilot::knowledge;
use ghostai_pilot::llm::{self, ChatMessage};
use tauri::State;

#[tauri::command]
pub fn get_knowledge_graph_stats(state: State<'_, AppState>) -> Result<KnowledgeGraphStats, String> {
    let km = state.knowledge_manager.lock().map_err(|e| e.to_string())?;

    let mut skills = 0;
    let mut experiences = 0;
    let mut star_stories = 0;
    let mut projects = 0;
    let mut companies = 0;

    if let Some(profile) = &km.personal_profile {
        skills = profile.skills.dominados.len() + profile.skills.intermedios.len();
        star_stories = profile.historias_star.len();
        experiences = profile.logros.len();
    }

    let skills_dir = format!("{}/skills", km.knowledge_base_path);
    if let Ok(entries) = std::fs::read_dir(&skills_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() { skills += 1; }
        }
    }

    let companies_dir = format!("{}/companies", km.knowledge_base_path);
    if let Ok(entries) = std::fs::read_dir(&companies_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() { companies += 1; }
        }
    }

    if let Some(profile) = &km.personal_profile {
        for skill in &profile.skills.dominados { projects += skill.proyectos.len(); }
        for skill in &profile.skills.intermedios { projects += skill.proyectos.len(); }
    }

    Ok(KnowledgeGraphStats { skills, experiences, star_stories, projects, companies })
}

#[tauri::command]
pub fn search_knowledge(query: String, state: State<'_, AppState>) -> Result<Vec<SearchResult>, String> {
    let km = state.knowledge_manager.lock().map_err(|e| e.to_string())?;
    let raw = km.search_all(&query);
    let results = raw.into_iter().map(|entry| {
        if let Some(space_idx) = entry.find(']') {
            SearchResult { provider: entry[1..space_idx].to_string(), content: entry[space_idx + 1..].trim().to_string() }
        } else {
            SearchResult { provider: "unknown".to_string(), content: entry }
        }
    }).collect();
    Ok(results)
}

#[tauri::command]
pub fn search_knowledge_fuzzy(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<FuzzySearchResult>, String> {
    let gp_lock = state.graph_provider.blocking_lock();
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let searcher = knowledge::search::EnhancedSearch::new(&graph);
    let opts = knowledge::search::SearchOptions { max_results: limit.unwrap_or(50), ..Default::default() };
    let results = searcher.search(&query, &opts).map_err(|e| e.to_string())?;
    Ok(results.into_iter().map(|r| FuzzySearchResult {
        entity_type: r.entity_type, entity_id: r.id, name: r.name,
        relevance_score: r.relevance_score,
        match_type: if r.relevance_score >= 0.99 { "exact".into() } else { "fuzzy".into() },
        matched_terms: r.matched_terms,
    }).collect())
}

#[tauri::command]
pub fn search_knowledge_enhanced(query: String, state: State<'_, AppState>) -> Result<Vec<EnhancedSearchResultIpc>, String> {
    let gp_lock = state.graph_provider.blocking_lock();
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let searcher = knowledge::search::EnhancedSearch::new(&graph);
    let options = knowledge::search::SearchOptions::default();
    let results = searcher.search(&query, &options).map_err(|e| format!("Enhanced search failed: {}", e))?;
    Ok(results.into_iter().map(|r| EnhancedSearchResultIpc {
        entity_type: r.entity_type, id: r.id, name: r.name,
        relevance_score: r.relevance_score, matched_terms: r.matched_terms, snippet: r.snippet,
    }).collect())
}

#[tauri::command]
pub async fn search_knowledge_semantic(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<SemanticSearchResult>, String> {
    let config = &state.config;
    let limit = limit.unwrap_or(10);

    if config.openai_api_key.is_empty() || config.openai_api_key == "ollama" {
        return Err("Embedding API not configured. Set OPENAI_API_KEY in config.".into());
    }

    let engine = knowledge::embeddings::EmbeddingEngine::new(config);
    let query_emb = engine.embed_single(&query).await.map_err(|e| format!("Embedding failed: {}", e))?;

    let (entity_texts, texts_to_embed): (std::collections::HashMap<String, (String, String, String)>, Vec<(String, String)>) = {
        let gp_lock = state.graph_provider.blocking_lock();
        let provider = gp_lock.as_ref().ok_or("Knowledge graph not initialized")?;
        let graph = provider.graph();
        let mut et = std::collections::HashMap::new();
        let mut tte = Vec::new();

        for skill in graph.list_skills().map_err(|e| e.to_string())? {
            let text = format!("{} {} {} {}", skill.name, skill.category.clone().unwrap_or_default(), "skill programming technology software development".to_string(), skill.level.clone());
            let id = skill.id.clone();
            et.insert(id.clone(), ("skill".to_string(), skill.name.clone(), skill.name.clone()));
            tte.push((id, text));
        }

        for story in graph.list_star_stories().map_err(|e| e.to_string())? {
            let text = format!("{} {} {} {} {}", story.title.clone().unwrap_or_default(), story.situation.clone(), story.task.clone(), story.action.clone(), story.result.clone());
            let id = story.id.clone();
            et.insert(id.clone(), ("star_story".to_string(), story.title.clone().unwrap_or_default(), story.title.clone().unwrap_or_default()));
            tte.push((id, text));
        }
        (et, tte)
    };

    let texts: Vec<String> = texts_to_embed.iter().map(|(_, t)| t.clone()).collect();
    let embeddings = engine.embed(texts).await.map_err(|e| e.to_string())?;

    let emb_map: std::collections::HashMap<String, Vec<f64>> = texts_to_embed.iter().zip(embeddings.into_iter()).map(|((id, _), emb)| (id.clone(), emb)).collect();
    let searcher = knowledge::embeddings::SemanticSearcher::new(&emb_map, &entity_texts);
    let results = searcher.search(&query_emb, limit);

    Ok(results.into_iter().map(|r| SemanticSearchResult {
        entity_type: r.entity_type, entity_id: r.entity_id, name: r.name,
        similarity: r.similarity, snippet: r.snippet,
    }).collect())
}

#[tauri::command]
pub async fn analyze_meeting(
    state: State<'_, AppState>,
    transcript: String,
    duration_minutes: Option<u32>,
) -> Result<serde_json::Value, String> {
    let config = state.config.clone();
    let duration_minutes = duration_minutes.unwrap_or(0);

    let existing_skills: Vec<String> = {
        let gp_lock = state.graph_provider.blocking_lock();
        let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
        let graph = provider.graph();
        let searcher = knowledge::search::KnowledgeSearcher::new(&*graph);
        searcher.context_search("").into_iter()
            .filter(|r| r.entity_type == "skill")
            .map(|r| r.name.to_lowercase())
            .collect()
    };

    let existing_str = if existing_skills.is_empty() { "No existing skills in profile".to_string() } else { existing_skills.join(", ") };

    let prompt = format!(
        "Analyze this meeting transcript and suggest skills/knowledge to learn.\n\nExisting skills: {}\nDuration: {} minutes\n\nTranscript:\n{}\n\nRespond in JSON with keys: suggestions (array of {{skill, category, reason}}), summary (string).",
        existing_str, duration_minutes, transcript
    );

    let response = llm::generate_response(&config, &vec![ChatMessage { role: "user".to_string(), content: prompt }])
        .await.map_err(|e| e.to_string())?;

    serde_json::to_value(serde_json::json!({
        "transcript": transcript, "suggestions": [], "summary": response, "duration_minutes": duration_minutes
    })).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_knowledge_context(query: String, state: State<'_, AppState>) -> Result<Vec<ContextResult>, String> {
    let gp_lock = state.graph_provider.blocking_lock();
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let searcher = knowledge::search::KnowledgeSearcher::new(&*graph);
    let results = searcher.context_search(&query);
    let filtered: Vec<ContextResult> = results.into_iter()
        .filter(|r| r.relevance_score > 0.5).take(5)
        .map(|r| ContextResult { entity_type: r.entity_type, name: r.name, relevance: r.relevance_score, snippet: r.snippet })
        .collect();
    Ok(filtered)
}

#[tauri::command]
pub fn find_relevant_stories(context: String, state: State<'_, AppState>) -> Result<Vec<StoryResult>, String> {
    let gp_lock = state.graph_provider.blocking_lock();
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let searcher = knowledge::search::KnowledgeSearcher::new(&*graph);
    let results = searcher.context_search(&context);
    let stories: Vec<StoryResult> = results.into_iter()
        .filter(|r| r.entity_type == "star_story").take(3)
        .map(|r| StoryResult { id: r.entity_id, title: r.name, tags: None, usage_count: 0, relevance: r.relevance_score })
        .collect();
    let enriched: Vec<StoryResult> = stories.into_iter().map(|mut s| {
        if let Ok(Some(story)) = graph.get_star_story(&s.id) { s.tags = story.tags; s.usage_count = story.usage_count; }
        s
    }).collect();
    Ok(enriched)
}

// ── STAR Matching ──────────────────────────────────────────────────────

/// Match STAR stories against an interview question or transcript context.
/// Uses a hybrid approach: hash-based embeddings + keyword search + graph edges.
#[tauri::command]
pub fn match_star_stories(
    context: String,
    state: State<'_, AppState>,
    max_results: Option<usize>,
    min_score: Option<f64>,
) -> Result<Vec<StarMatchResult>, String> {
    let gp_lock = state.graph_provider.blocking_lock();
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();

    let matcher = knowledge::star_matcher::StarMatcher::new(&graph);
    let opts = knowledge::star_matcher::StarMatchOptions {
        max_results: max_results.unwrap_or(5),
        min_score: min_score.unwrap_or(0.15),
        ..Default::default()
    };

    let matches = matcher
        .match_stories(&context, Some(opts))
        .map_err(|e| format!("STAR matching failed: {}", e))?;

    Ok(matches
        .into_iter()
        .map(|m| StarMatchResult {
            story: m.story.into(),
            relevance_score: m.relevance_score,
            embedding_similarity: m.embedding_similarity,
            keyword_score: m.keyword_score,
            edge_boost: m.edge_boost,
            linked_skills: m
                .linked_skills
                .into_iter()
                .map(|e| LinkedEntityInfo {
                    id: e.id,
                    name: e.name,
                    relation: e.relation,
                    weight: e.weight,
                })
                .collect(),
            linked_projects: m
                .linked_projects
                .into_iter()
                .map(|e| LinkedEntityInfo {
                    id: e.id,
                    name: e.name,
                    relation: e.relation,
                    weight: e.weight,
                })
                .collect(),
            linked_companies: m
                .linked_companies
                .into_iter()
                .map(|e| LinkedEntityInfo {
                    id: e.id,
                    name: e.name,
                    relation: e.relation,
                    weight: e.weight,
                })
                .collect(),
            matched_terms: m.matched_terms,
        })
        .collect())
}

/// Match STAR stories by specific tags (skills, categories)
#[tauri::command]
pub fn match_star_stories_by_tags(
    tags: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<StarMatchResult>, String> {
    let gp_lock = state.graph_provider.blocking_lock();
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();

    let matcher = knowledge::star_matcher::StarMatcher::new(&graph);
    let matches = matcher
        .match_by_tags(&tags)
        .map_err(|e| format!("STAR tag matching failed: {}", e))?;

    Ok(matches
        .into_iter()
        .map(|m| StarMatchResult {
            story: m.story.into(),
            relevance_score: m.relevance_score,
            embedding_similarity: m.embedding_similarity,
            keyword_score: m.keyword_score,
            edge_boost: m.edge_boost,
            linked_skills: m
                .linked_skills
                .into_iter()
                .map(|e| LinkedEntityInfo {
                    id: e.id,
                    name: e.name,
                    relation: e.relation,
                    weight: e.weight,
                })
                .collect(),
            linked_projects: m
                .linked_projects
                .into_iter()
                .map(|e| LinkedEntityInfo {
                    id: e.id,
                    name: e.name,
                    relation: e.relation,
                    weight: e.weight,
                })
                .collect(),
            linked_companies: m
                .linked_companies
                .into_iter()
                .map(|e| LinkedEntityInfo {
                    id: e.id,
                    name: e.name,
                    relation: e.relation,
                    weight: e.weight,
                })
                .collect(),
            matched_terms: m.matched_terms,
        })
        .collect())
}

/// Record that a STAR story was used (increments usage_count)
#[tauri::command]
pub fn record_star_story_usage(
    story_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let gp_lock = state.graph_provider.blocking_lock();
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();

    let matcher = knowledge::star_matcher::StarMatcher::new(&graph);
    matcher
        .record_usage(&story_id)
        .map_err(|e| format!("Failed to record usage: {}", e))?;
    Ok(())
}