// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ghostai_pilot::{audio, audio_config, config, knowledge, llm, loopback, stt, vad};
use ghostai_pilot::audio_config::AudioStrategy;
use ghostai_pilot::knowledge::cv_parser;
use ghostai_pilot::knowledge::graph::GraphKnowledgeProvider;
use ghostai_pilot::knowledge::graph::KnowledgeGraph;
use ghostai_pilot::knowledge::migration::{self, MigrationResult};
use ghostai_pilot::llm::ChatMessage;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, State, WebviewWindow, WebviewWindowBuilder, WebviewUrl, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use cpal::traits::{DeviceTrait, HostTrait};

// Wrapper to make cpal::Stream Send + Sync
struct StreamWrapper(cpal::Stream);
unsafe impl Send for StreamWrapper {}
unsafe impl Sync for StreamWrapper {}

#[derive(Serialize, Clone)]
struct TranscriptionPayload {
    text: String,
}

#[derive(Serialize, Clone)]
struct SuggestionPayload {
    text: String,
}

struct AppState {
    pub config: config::Config,
    pub knowledge_manager: Arc<Mutex<knowledge::personal::KnowledgeManager>>,
    pub graph_provider: Arc<Mutex<Option<GraphKnowledgeProvider>>>,
    pub is_locked: Arc<Mutex<bool>>,
    pub conversation: Arc<Mutex<Vec<ChatMessage>>>,
    pub active_streams: Arc<Mutex<Vec<StreamWrapper>>>,
}

#[derive(Serialize, Clone)]
struct AudioDevice {
    index: usize,
    name: String,
    label: String,
}

#[tauri::command]
fn get_audio_processes() -> Result<Vec<loopback::real::AudioProcess>, String> {
    Ok(loopback::real::list_audio_processes())
}

#[tauri::command]
fn get_audio_devices() -> Result<Vec<AudioDevice>, String> {
    let host = cpal::default_host();
    let mut result = Vec::new();
    if let Ok(input_devices) = host.input_devices() {
        for (i, device) in input_devices.enumerate() {
            let name = device.name().unwrap_or_else(|_| format!("Device {}", i));
            result.push(AudioDevice {
                index: i,
                label: if name.to_lowercase().contains("voicemeeter") { "🔊 VoiceMeeter" } else { "🎤 Input" }.to_string(),
                name,
            });
        }
    }
    Ok(result)
}

#[tauri::command]
fn set_lock_state(window: WebviewWindow, state: State<'_, AppState>, locked: bool) -> Result<(), String> {
    let mut is_locked = state.is_locked.lock().map_err(|e| e.to_string())?;
    *is_locked = locked;
    let _ = window.set_ignore_cursor_events(locked);
    Ok(())
}

#[tauri::command]
fn clear_feed(state: State<'_, AppState>) -> Result<(), String> {
    let mut conversation = state.conversation.lock().map_err(|e| e.to_string())?;
    if !conversation.is_empty() {
        conversation.truncate(1);
    }
    Ok(())
}

#[tauri::command]
fn regenerate(app_handle: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let conversation = state.conversation.lock().map_err(|e| e.to_string())?.clone();
    let config = state.config.clone();
    if conversation.len() < 2 { return Err("No data".to_string()); }

    let app = app_handle.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
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
async fn start_capture(app_handle: AppHandle, state: State<'_, AppState>, pid: Option<u32>, device_index: Option<usize>) -> Result<(), String> {
    let config = state.config.clone();
    let km_lock = state.knowledge_manager.clone();
    let conversation_lock = state.conversation.clone();
    let streams_lock = state.active_streams.clone();

    {
        let mut streams = streams_lock.lock().map_err(|e| e.to_string())?;
        streams.clear();
    }

    let (audio_rx, stream) = if let Some(p) = pid {
        #[cfg(feature = "wasapi_loopback")]
        {
            use ghostai_pilot::audio_config::WindowsStrategy;
            let strategy = WindowsStrategy::new().with_process(p);
            let rx = strategy.start_system_capture().map_err(|e| e.to_string())?;
            (rx, None)
        }
        #[cfg(not(feature = "wasapi_loopback"))]
        {
            let strategy = audio_config::detect_strategy().map_err(|e| e.to_string())?;
            let rx = strategy.start_system_capture().map_err(|e| e.to_string())?;
            (rx, None)
        }
    } else {
        let host = cpal::default_host();
        let devices: Vec<_> = host.input_devices().map_err(|e| e.to_string())?.collect();
        let device = if let Some(idx) = device_index {
            devices.get(idx).ok_or_else(|| "Invalid index".to_string())?.clone()
        } else {
            audio::find_microphone_device().map_err(|e| e.to_string())?
        };
        println!("  ⚙ Captura: {:?}", device.name().unwrap_or_default());
        let (rx, stream) = audio::start_capture(device).map_err(|e| e.to_string())?;
        (rx, Some(stream))
    };

    if let Some(s) = stream {
        let mut streams = streams_lock.lock().map_err(|e| e.to_string())?;
        streams.push(StreamWrapper(s));
    }

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut vad_detector = vad::VadDetector::default_config();
        
        {
            let mut conversation = conversation_lock.lock().unwrap();
            if conversation.is_empty() {
                let km = km_lock.lock().unwrap();
                let system_prompt = knowledge::personal::generate_system_prompt(&km);
                conversation.push(ChatMessage { role: "system".to_string(), content: system_prompt });
            }
        }

        let mut speech_buffer: Vec<f32> = Vec::with_capacity(16000 * 10);
        let mut is_capturing = false;
        
        while let Ok(chunk) = audio_rx.recv() {
            let vad_event = vad_detector.process(&chunk.samples);
            match vad_event {
                vad::VadEvent::SpeechStart => {
                    is_capturing = true;
                    speech_buffer.clear();
                    speech_buffer.extend_from_slice(&chunk.samples);
                }
                vad::VadEvent::SpeechEnd { .. } => {
                    if is_capturing && !speech_buffer.is_empty() {
                        is_capturing = false;
                        let wav_bytes = match stt::pcm_to_wav(&speech_buffer, chunk.sample_rate) {
                            Ok(bytes) => bytes,
                            Err(e) => {
                                eprintln!("WAV encoding failed: {}", e);
                                continue;
                            }
                        };
                        if speech_buffer.len() < 8000 { continue; }

                        if let Ok(transcription) = rt.block_on(stt::transcribe(&config, &wav_bytes)) {
                            if transcription.trim().is_empty() { continue; }
                            println!("  📝 \"{}\"", transcription);
                            
                            // EMIT DIRECTLY TO WINDOW WITH DIAGNOSTICS
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.emit("transcription-update", TranscriptionPayload { text: transcription.clone() });
                            } else {
                                println!("  ❌ ERROR: Ventana 'main' no encontrada para transcripción!");
                                // Fallback a emit global
                                let _ = app_handle.emit("transcription-update", TranscriptionPayload { text: transcription.clone() });
                            }

                            let (relevant_stories, external_knowledge) = {
                                let km = km_lock.lock().unwrap();
                                let stories: Vec<_> = if let Some(profile) = &km.personal_profile {
                                    profile.find_relevant_stories(&transcription).into_iter().cloned().collect()
                                } else {
                                    vec![]
                                };
                                let ext = km.search_all(&transcription);
                                (stories, ext)
                            }; // km lock released here

                            let mut conversation = conversation_lock.lock().unwrap();

                            // Truncate conversation to last 20 messages (keep system prompt at index 0)
                            const MAX_CONVERSATION_LEN: usize = 21; // 1 system + 20 messages
                            if conversation.len() > MAX_CONVERSATION_LEN {
                                let excess = conversation.len() - (MAX_CONVERSATION_LEN - 1);
                                conversation.drain(1..1 + excess);
                                // Ensure system prompt is preserved
                                if conversation[0].role != "system" {
                                    // system msg already at index 0 from .first().cloned()
                                }
                            }

                            if !relevant_stories.is_empty() || !external_knowledge.is_empty() {
                                let mut context_msg = String::from("RELEVANT CONTEXT FOUND:\n");
                                for story in relevant_stories {
                                    context_msg.push_str(&format!("- STAR STORY [{}]: {} -> {}\n",
                                        story.id, story.situacion, story.resultado));
                                }
                                for ext in external_knowledge {
                                    context_msg.push_str(&format!("- EXTERNAL: {}\n", ext));
                                }

                                conversation.push(ChatMessage {
                                    role: "system".to_string(),
                                    content: context_msg,
                                });
                            }

                            conversation.push(ChatMessage { role: "user".to_string(), content: transcription });

                            if let Ok(response) = rt.block_on(llm::generate_response(&config, &conversation)) {
                                println!("  🤖 IA: {}", response);
                                if let Some(window) = app_handle.get_webview_window("main") {
                                    let _ = window.emit("suggestion-update", SuggestionPayload { text: response.clone() });
                                } else {
                                    println!("  ❌ ERROR: Ventana 'main' no encontrada para sugerencia!");
                                    let _ = app_handle.emit("suggestion-update", SuggestionPayload { text: response.clone() });
                                }
                                conversation.push(ChatMessage { role: "assistant".to_string(), content: response });
                            }
                        }
                    }
                }
                vad::VadEvent::Silence => {
                    if is_capturing { speech_buffer.extend_from_slice(&chunk.samples); }
                }
            }
        }
    });
    Ok(())
}

#[derive(Serialize, Clone)]
struct KnowledgeGraphStats {
    skills: usize,
    experiences: usize,
    star_stories: usize,
    projects: usize,
    companies: usize,
}

#[tauri::command]
fn open_profile_window(app_handle: AppHandle) -> Result<(), String> {
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

#[tauri::command]
fn get_knowledge_graph_stats(state: State<'_, AppState>) -> Result<KnowledgeGraphStats, String> {
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
            if entry.path().is_dir() {
                skills += 1;
            }
        }
    }

    let companies_dir = format!("{}/companies", km.knowledge_base_path);
    if let Ok(entries) = std::fs::read_dir(&companies_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                companies += 1;
            }
        }
    }

    if let Some(profile) = &km.personal_profile {
        for skill in &profile.skills.dominados {
            projects += skill.proyectos.len();
        }
        for skill in &profile.skills.intermedios {
            projects += skill.proyectos.len();
        }
    }

    Ok(KnowledgeGraphStats {
        skills,
        experiences,
        star_stories,
        projects,
        companies,
    })
}

#[derive(Serialize)]
struct SearchResult {
  provider: String,
  content: String,
}

#[derive(Serialize, Clone)]
struct ExperienceWithSkills {
    id: String,
    company: String,
    role: String,
    start_date: String,
    end_date: Option<String>,
    description: Option<String>,
    highlights: Option<String>,
    skills: Vec<String>,
}

#[tauri::command]
fn list_experiences_with_skills(state: State<'_, AppState>) -> Result<Vec<ExperienceWithSkills>, String> {
    let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();

    let experiences = graph.list_experiences().map_err(|e| e.to_string())?;
    let mut result = Vec::new();

    for exp in experiences {
        let edges = graph.get_edges_for_entity(&exp.id, ghostai_pilot::knowledge::graph::EntityType::Experience)
            .map_err(|e| e.to_string())?;

        let mut skill_names = Vec::new();
        for edge in &edges {
            let connected_id = if edge.source_id == exp.id {
                &edge.target_id
            } else {
                &edge.source_id
            };
            if let Ok(Some(skill)) = graph.get_skill(connected_id) {
                skill_names.push(skill.name);
            }
        }

        result.push(ExperienceWithSkills {
            id: exp.id,
            company: exp.company,
            role: exp.role,
            start_date: exp.start_date,
            end_date: exp.end_date,
            description: exp.description,
            highlights: exp.highlights,
            skills: skill_names,
        });
    }

    Ok(result)
}

// ─── Skill types for IPC ─────────────────────────────────────────────────

#[derive(Serialize, Clone, Deserialize)]
struct SkillData {
    id: Option<String>,
    name: String,
    category: Option<String>,
    level: String,
    years: i32,
}

#[derive(Serialize, Clone)]
struct SkillRecord {
    id: String,
    name: String,
    category: Option<String>,
    level: String,
    years: i32,
    source: Option<String>,
    created_at: String,
    updated_at: String,
}

impl From<ghostai_pilot::knowledge::graph::SkillEntity> for SkillRecord {
    fn from(e: ghostai_pilot::knowledge::graph::SkillEntity) -> Self {
        Self {
            id: e.id,
            name: e.name,
            category: e.category,
            level: e.level,
            years: e.years,
            source: e.source,
            created_at: e.created_at,
            updated_at: e.updated_at,
        }
    }
}

// ─── Skill CRUD commands ─────────────────────────────────────────────────

#[tauri::command]
fn list_skills(state: State<'_, AppState>) -> Result<Vec<SkillRecord>, String> {
    let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let skills = graph.list_skills().map_err(|e| format!("Failed to list skills: {}", e))?;
    Ok(skills.into_iter().map(SkillRecord::from).collect())
}

#[tauri::command]
fn create_skill(state: State<'_, AppState>, data: SkillData) -> Result<SkillRecord, String> {
    let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let entity = graph.create_skill(
        &data.name,
        data.category.as_deref(),
        &data.level,
        data.years,
        None,
    ).map_err(|e| format!("Failed to create skill: {}", e))?;
    Ok(entity.into())
}

#[tauri::command]
fn update_skill(state: State<'_, AppState>, data: SkillData) -> Result<SkillRecord, String> {
    let id = data.id.as_ref().ok_or_else(|| "Skill ID required for update".to_string())?;
    let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();

    let existing = graph.get_skill(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Skill {} not found", id))?;

    let updated = ghostai_pilot::knowledge::graph::SkillEntity {
        id: existing.id,
        name: data.name,
        category: data.category,
        level: data.level,
        years: data.years,
        source: existing.source,
        created_at: existing.created_at,
        updated_at: existing.updated_at,
    };

    graph.update_skill(&updated).map_err(|e| format!("Failed to update skill: {}", e))?;
    Ok(updated.into())
}

#[tauri::command]
fn delete_skill(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    graph.delete_skill(&id).map_err(|e| format!("Failed to delete skill: {}", e))
}

// ─── Experience CRUD commands ─────────────────────────────────────────────

#[derive(Serialize, Clone, Deserialize)]
struct ExperienceData {
    id: Option<String>,
    company: String,
    role: String,
    start_date: String,
    end_date: Option<String>,
    description: Option<String>,
    highlights: Option<String>,
    skill_ids: Option<Vec<String>>,
}

#[tauri::command]
fn create_experience(state: State<'_, AppState>, data: ExperienceData) -> Result<ExperienceWithSkills, String> {
    let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();

    let entity = graph.create_experience(
        &data.company,
        &data.role,
        &data.start_date,
        data.end_date.as_deref(),
        data.description.as_deref(),
        data.highlights.as_deref(),
    ).map_err(|e| format!("Failed to create experience: {}", e))?;

    let mut skill_names = Vec::new();
    if let Some(skill_ids) = &data.skill_ids {
        for skill_id in skill_ids {
            let _ = graph.add_edge(
                &entity.id,
                ghostai_pilot::knowledge::graph::EntityType::Experience,
                skill_id,
                ghostai_pilot::knowledge::graph::EntityType::Skill,
                "used",
                1.0,
            );
            if let Ok(Some(skill)) = graph.get_skill(skill_id) {
                skill_names.push(skill.name);
            }
        }
    }

    Ok(ExperienceWithSkills {
        id: entity.id,
        company: entity.company,
        role: entity.role,
        start_date: entity.start_date,
        end_date: entity.end_date,
        description: entity.description,
        highlights: entity.highlights,
        skills: skill_names,
    })
}

#[tauri::command]
fn update_experience(state: State<'_, AppState>, data: ExperienceData) -> Result<ExperienceWithSkills, String> {
    let id = data.id.as_ref().ok_or_else(|| "Experience ID required for update".to_string())?;
    let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();

    let existing = graph.get_experience(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Experience {} not found", id))?;

    let updated = ghostai_pilot::knowledge::graph::ExperienceEntity {
        id: existing.id,
        company: data.company,
        role: data.role,
        start_date: data.start_date,
        end_date: data.end_date,
        description: data.description,
        highlights: data.highlights,
    };

    graph.update_experience(&updated).map_err(|e| format!("Failed to update experience: {}", e))?;

    if let Some(skill_ids) = &data.skill_ids {
        let old_edges = graph.get_edges_for_entity(id, ghostai_pilot::knowledge::graph::EntityType::Experience)
            .map_err(|e| e.to_string())?;
        for edge in old_edges {
            if edge.target_type == "skill" || edge.source_type == "skill" {
                let _ = graph.remove_edge(&edge.id);
            }
        }
        for skill_id in skill_ids {
            let _ = graph.add_edge(
                id,
                ghostai_pilot::knowledge::graph::EntityType::Experience,
                skill_id,
                ghostai_pilot::knowledge::graph::EntityType::Skill,
                "used",
                1.0,
            );
        }
    }

    let mut skill_names = Vec::new();
    let edges = graph.get_edges_for_entity(id, ghostai_pilot::knowledge::graph::EntityType::Experience)
        .map_err(|e| e.to_string())?;
    for edge in &edges {
        let connected_id = if edge.source_id == *id { &edge.target_id } else { &edge.source_id };
        if let Ok(Some(skill)) = graph.get_skill(connected_id) {
            skill_names.push(skill.name);
        }
    }

    Ok(ExperienceWithSkills {
        id: updated.id,
        company: updated.company,
        role: updated.role,
        start_date: updated.start_date,
        end_date: updated.end_date,
        description: updated.description,
        highlights: updated.highlights,
        skills: skill_names,
    })
}

#[tauri::command]
fn delete_experience(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();

    let edges = graph.get_edges_for_entity(&id, ghostai_pilot::knowledge::graph::EntityType::Experience)
        .map_err(|e| e.to_string())?;
    for edge in edges {
        let _ = graph.remove_edge(&edge.id);
    }

    graph.delete_experience(&id).map_err(|e| format!("Failed to delete experience: {}", e))
}

// ─── Edge commands ────────────────────────────────────────────────────────

#[derive(Serialize, Clone, Deserialize)]
struct EdgeRecord {
    id: String,
    source_id: String,
    source_type: String,
    target_id: String,
    target_type: String,
    relation: String,
    weight: f64,
}

impl From<ghostai_pilot::knowledge::graph::Edge> for EdgeRecord {
    fn from(e: ghostai_pilot::knowledge::graph::Edge) -> Self {
        Self {
            id: e.id,
            source_id: e.source_id,
            source_type: e.source_type,
            target_id: e.target_id,
            target_type: e.target_type,
            relation: e.relation,
            weight: e.weight,
        }
    }
}

fn parse_entity_type(s: &str) -> Result<ghostai_pilot::knowledge::graph::EntityType, String> {
    match s {
        "skill" => Ok(ghostai_pilot::knowledge::graph::EntityType::Skill),
        "experience" => Ok(ghostai_pilot::knowledge::graph::EntityType::Experience),
        "project" => Ok(ghostai_pilot::knowledge::graph::EntityType::Project),
        "company" => Ok(ghostai_pilot::knowledge::graph::EntityType::Company),
        "star_story" => Ok(ghostai_pilot::knowledge::graph::EntityType::StarStory),
        _ => Err(format!("Unknown entity type: {}", s)),
    }
}

#[tauri::command]
fn add_edge(
    state: State<'_, AppState>,
    source_id: String,
    source_type: String,
    target_id: String,
    target_type: String,
    relation: String,
    weight: f64,
) -> Result<EdgeRecord, String> {
    let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let src_t = parse_entity_type(&source_type)?;
    let tgt_t = parse_entity_type(&target_type)?;
    let edge = graph.add_edge(&source_id, src_t, &target_id, tgt_t, &relation, weight)
        .map_err(|e| format!("Failed to add edge: {}", e))?;
    Ok(edge.into())
}

#[tauri::command]
fn remove_edge(state: State<'_, AppState>, edge_id: String) -> Result<bool, String> {
    let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    graph.remove_edge(&edge_id).map_err(|e| format!("Failed to remove edge: {}", e))
}

#[tauri::command]
fn list_edges_for_entity(
    state: State<'_, AppState>,
    entity_id: String,
    entity_type: String,
) -> Result<Vec<EdgeRecord>, String> {
    let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let et = parse_entity_type(&entity_type)?;
    let edges = graph.get_edges_for_entity(&entity_id, et)
        .map_err(|e| format!("Failed to list edges: {}", e))?;
    Ok(edges.into_iter().map(EdgeRecord::from).collect())
}

// ─── STAR Story types for IPC ───────────────────────────────────────────

#[derive(Serialize, Clone, Deserialize)]
struct StarStoryData {
    id: Option<String>,
    title: Option<String>,
    situation: String,
    task: String,
    action: String,
    result: String,
    tags: Option<String>,
    difficulty: Option<String>,
    stakes: Option<String>,
}

#[derive(Serialize, Clone)]
struct StarStoryRecord {
    id: String,
    title: Option<String>,
    situation: String,
    task: String,
    action: String,
    result: String,
    tags: Option<String>,
    difficulty: Option<String>,
    stakes: Option<String>,
    usage_count: i32,
    created_at: String,
    updated_at: String,
}

impl From<ghostai_pilot::knowledge::graph::StarStoryEntity> for StarStoryRecord {
    fn from(e: ghostai_pilot::knowledge::graph::StarStoryEntity) -> Self {
        Self {
            id: e.id,
            title: e.title,
            situation: e.situation,
            task: e.task,
            action: e.action,
            result: e.result,
            tags: e.tags,
            difficulty: e.difficulty,
            stakes: e.stakes,
            usage_count: e.usage_count,
            created_at: e.created_at,
            updated_at: e.updated_at,
        }
    }
}

// ─── STAR Story CRUD commands ───────────────────────────────────────────

#[tauri::command]
fn create_star_story(state: State<'_, AppState>, data: StarStoryData) -> Result<StarStoryRecord, String> {
    let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let entity = graph.create_star_story(
        data.title.as_deref(),
        &data.situation,
        &data.task,
        &data.action,
        &data.result,
        data.tags.as_deref(),
        data.difficulty.as_deref(),
        data.stakes.as_deref(),
    ).map_err(|e| format!("Failed to create STAR story: {}", e))?;
    Ok(entity.into())
}

#[tauri::command]
fn update_star_story(state: State<'_, AppState>, data: StarStoryData) -> Result<StarStoryRecord, String> {
    let id = data.id.as_ref().ok_or_else(|| "Story ID required for update".to_string())?;
    let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();

    let existing = graph.get_star_story(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("STAR story {} not found", id))?;

    let updated = ghostai_pilot::knowledge::graph::StarStoryEntity {
        id: existing.id,
        title: data.title,
        situation: data.situation,
        task: data.task,
        action: data.action,
        result: data.result,
        tags: data.tags,
        difficulty: data.difficulty,
        stakes: data.stakes,
        usage_count: existing.usage_count,
        created_at: existing.created_at,
        updated_at: existing.updated_at,
    };

    graph.update_star_story(&updated).map_err(|e| format!("Failed to update STAR story: {}", e))?;
    Ok(updated.into())
}

#[tauri::command]
fn delete_star_story(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    graph.delete_star_story(&id).map_err(|e| format!("Failed to delete STAR story: {}", e))
}

#[tauri::command]
fn get_star_stories(state: State<'_, AppState>) -> Result<Vec<StarStoryRecord>, String> {
    let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let stories = graph.list_star_stories().map_err(|e| format!("Failed to list STAR stories: {}", e))?;
    Ok(stories.into_iter().map(StarStoryRecord::from).collect())
}

#[tauri::command]
async fn coach_star_story(
    state: State<'_, AppState>,
    story_id: Option<String>,
    question: String,
) -> Result<String, String> {
    let config = state.config.clone();

    let story_context = if let Some(sid) = story_id {
        let result = {
            let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
            let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
            let graph = provider.graph();
            let story = graph.get_star_story(&sid).map_err(|e| e.to_string())?;
            story.map(|s| format!(
                "Here is the user's STAR story:\nTitle: {}\nSituation: {}\nTask: {}\nAction: {}\nResult: {}\nTags: {}\nDifficulty: {}\nStakes: {}",
                s.title.as_deref().unwrap_or("Untitled"),
                s.situation, s.task, s.action, s.result,
                s.tags.as_deref().unwrap_or("none"),
                s.difficulty.as_deref().unwrap_or("unspecified"),
                s.stakes.as_deref().unwrap_or("unspecified"),
            ))
        }; // gp_lock dropped here
        result
    } else {
        None
    };

    let system_msg = ChatMessage {
        role: "system".to_string(),
        content: "You are an expert interview coach specializing in the STAR method (Situation, Task, Action, Result). \
                  Help the user improve their STAR stories for behavioral interviews. \
                  Provide specific, actionable suggestions. Be concise (2-4 sentences max per suggestion). \
                  Focus on: specificity, quantifiable results, personal impact, and clarity of narrative.".to_string(),
    };

    let mut messages = vec![system_msg];

    if let Some(ctx) = story_context {
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: ctx,
        });
    }

    messages.push(ChatMessage {
        role: "user".to_string(),
        content: question,
    });

    let response = llm::generate_response_with_options(&config, &messages, 800)
        .await
        .map_err(|e| format!("LLM coaching error: {}", e))?;

    Ok(response)
}

#[tauri::command]
fn search_knowledge(query: String, state: State<'_, AppState>) -> Result<Vec<SearchResult>, String> {
    let km = state.knowledge_manager.lock().map_err(|e| e.to_string())?;
    let raw = km.search_all(&query);
    let results = raw
        .into_iter()
        .map(|entry| {
            if let Some(space_idx) = entry.find(']') {
                let provider = entry[1..space_idx].to_string();
                let content = entry[space_idx + 1..].trim().to_string();
                SearchResult { provider, content }
            } else {
                SearchResult {
                    provider: "unknown".to_string(),
                    content: entry,
                }
            }
        })
        .collect();
    Ok(results)
}

#[derive(Serialize, Clone)]
struct FuzzySearchResult {
    entity_type: String,
    entity_id: String,
    name: String,
    relevance_score: f64,
    match_type: String,
    matched_terms: Vec<String>,
}

#[tauri::command]
fn search_knowledge_fuzzy(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<FuzzySearchResult>, String> {
    let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    let searcher = knowledge::search::EnhancedSearch::new(&graph);
    let opts = knowledge::search::SearchOptions {
        max_results: limit.unwrap_or(50),
        ..Default::default()
    };
    let results = searcher.search(&query, &opts).map_err(|e| e.to_string())?;
    Ok(results
        .into_iter()
        .map(|r| FuzzySearchResult {
            entity_type: r.entity_type,
            entity_id: r.id,
            name: r.name,
            relevance_score: r.relevance_score,
            match_type: if r.relevance_score >= 0.99 { "exact".into() } else { "fuzzy".into() },
            matched_terms: r.matched_terms,
        })
        .collect())
}

#[tauri::command]
async fn analyze_meeting(
    state: State<'_, AppState>,
    transcript: String,
    duration_minutes: Option<u32>,
) -> Result<serde_json::Value, String> {
    let config = state.config.clone();
    let duration_minutes = duration_minutes.unwrap_or(0);

    // Extract existing skill names synchronously (graph contains non-Send rusqlite)
    let existing_skills: Vec<String> = {
        let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
        let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
        let graph = provider.graph();
        let searcher = knowledge::search::KnowledgeSearcher::new(&*graph);
        searcher.context_search("")
            .into_iter()
            .filter(|r| r.entity_type == "skill")
            .map(|r| r.name.to_lowercase())
            .collect()
    };

    // Build prompt and call LLM directly (no non-Send types held across await)
    let existing_str = if existing_skills.is_empty() {
        "No existing skills in profile".to_string()
    } else {
        existing_skills.join(", ")
    };

    let prompt = format!(
        "Analyze this meeting transcript and suggest skills/knowledge to learn.\n\n\
         Existing skills: {}\n\
         Duration: {} minutes\n\n\
         Transcript:\n{}\n\n\
         Respond in JSON with keys: suggestions (array of {{skill, category, reason}}), summary (string).",
        existing_str, duration_minutes, transcript
    );

    let response = llm::generate_response(&config, &vec![ChatMessage {
        role: "user".to_string(),
        content: prompt,
    }])
    .await
    .map_err(|e| e.to_string())?;

    serde_json::to_value(serde_json::json!({
        "transcript": transcript,
        "suggestions": [],
        "summary": response,
        "duration_minutes": duration_minutes
    })).map_err(|e| e.to_string())
}

#[derive(Serialize, Clone)]
struct EnhancedSearchResultIpc {
    entity_type: String,
    id: String,
    name: String,
    relevance_score: f64,
    matched_terms: Vec<String>,
    snippet: String,
}

#[tauri::command]
fn search_knowledge_enhanced(query: String, state: State<'_, AppState>) -> Result<Vec<EnhancedSearchResultIpc>, String> {
    let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();

    let searcher = knowledge::search::EnhancedSearch::new(&graph);
    let options = knowledge::search::SearchOptions::default();
    let results = searcher.search(&query, &options)
        .map_err(|e| format!("Enhanced search failed: {}", e))?;

    Ok(results
        .into_iter()
        .map(|r| EnhancedSearchResultIpc {
            entity_type: r.entity_type,
            id: r.id,
            name: r.name,
            relevance_score: r.relevance_score,
            matched_terms: r.matched_terms,
            snippet: r.snippet,
        })
        .collect())
}

#[derive(Serialize, Clone)]
struct SemanticSearchResult {
    entity_type: String,
    entity_id: String,
    name: String,
    similarity: f64,
    snippet: String,
}

#[tauri::command]
async fn search_knowledge_semantic(
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

    let query_emb = engine.embed_single(&query)
        .await
        .map_err(|e| format!("Embedding failed: {}", e))?;

    // Extract all graph data synchronously, drop lock before any .await
    let (entity_texts, texts_to_embed): (std::collections::HashMap<String, (String, String, String)>, Vec<(String, String)>) = {
        let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
        let provider = gp_lock.as_ref().ok_or("Knowledge graph not initialized")?;
        let graph = provider.graph();

        let mut et: std::collections::HashMap<String, (String, String, String)> = std::collections::HashMap::new();
        let mut tte: Vec<(String, String)> = Vec::new();

        for skill in graph.list_skills().map_err(|e| e.to_string())? {
            let text = format!("{} {} {} {}",
                skill.name,
                skill.category.clone().unwrap_or_default(),
                "skill programming technology software development".to_string(),
                skill.level.clone()
            );
            let id = skill.id.clone();
            et.insert(id.clone(), ("skill".to_string(), skill.name.clone(), skill.name.clone()));
            tte.push((id, text));
        }

        for story in graph.list_star_stories().map_err(|e| e.to_string())? {
            let text = format!("{} {} {} {} {}",
                story.title.clone().unwrap_or_default(),
                story.situation.clone(),
                story.task.clone(),
                story.action.clone(),
                story.result.clone()
            );
            let id = story.id.clone();
            et.insert(id.clone(), ("star_story".to_string(), story.title.clone().unwrap_or_default(), story.title.clone().unwrap_or_default()));
            tte.push((id, text));
        }
        (et, tte)
    }; // gp_lock dropped here

    let texts: Vec<String> = texts_to_embed.iter().map(|(_, t)| t.clone()).collect();
    let embeddings = engine.embed(texts).await.map_err(|e| e.to_string())?;

    let emb_map: std::collections::HashMap<String, Vec<f64>> = texts_to_embed
        .iter()
        .zip(embeddings.into_iter())
        .map(|((id, _), emb)| (id.clone(), emb))
        .collect();

    let searcher = knowledge::embeddings::SemanticSearcher::new(&emb_map, &entity_texts);
    let results = searcher.search(&query_emb, limit);

    Ok(results
        .into_iter()
        .map(|r| SemanticSearchResult {
            entity_type: r.entity_type,
            entity_id: r.entity_id,
            name: r.name,
            similarity: r.similarity,
            snippet: r.snippet,
        })
        .collect())
}

#[derive(Serialize, Clone)]
struct CvPreview {
    parsed: cv_parser::ParsedCv,
    file_name: String,
    text_length: usize,
}

#[tauri::command]
async fn parse_cv(
    state: State<'_, AppState>,
    file_path: String,
) -> Result<CvPreview, String> {
    let path = std::path::Path::new(&file_path);
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let text = cv_parser::extract_text(path)
        .map_err(|e| format!("Failed to extract text from CV: {}", e))?;

    let text_length = text.len();

    let parsed_config = state.config.clone();
    let parsed = cv_parser::parse_cv_with_llm(&parsed_config, &text)
        .await
        .map_err(|e| format!("Failed to parse CV with AI: {}", e))?;

    Ok(CvPreview {
        parsed,
        file_name,
        text_length,
    })
}

#[tauri::command]
fn confirm_cv_import(
    state: State<'_, AppState>,
    parsed: cv_parser::ParsedCv,
) -> Result<cv_parser::CvImportResult, String> {
    let graph_provider_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;

    let provider = graph_provider_lock
        .as_ref()
        .ok_or_else(|| "Knowledge graph not initialized".to_string())?;

    let graph = provider.graph();
    let result = cv_parser::import_parsed_cv(&graph, &parsed)
        .map_err(|e| format!("Failed to import CV data: {}", e))?;

    Ok(result)
}

#[tauri::command]
async fn open_cv_dialog(app_handle: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let file_path = app_handle
        .dialog()
        .file()
        .add_filter("CV Files", &["pdf", "txt", "md"])
        .set_title("Select your CV / Resume")
        .blocking_pick_file()
        .map(|p| {
            match p {
                tauri_plugin_dialog::FilePath::Path(pathbuf) => pathbuf.to_string_lossy().to_string(),
                tauri_plugin_dialog::FilePath::Url(url) => url.to_string(),
            }
        });

    Ok(file_path)
}

#[derive(Serialize, Clone)]
struct CompanyRecord {
    id: String,
    name: String,
    industry: Option<String>,
    description: Option<String>,
}

impl From<ghostai_pilot::knowledge::graph::CompanyEntity> for CompanyRecord {
    fn from(e: ghostai_pilot::knowledge::graph::CompanyEntity) -> Self {
        Self {
            id: e.id,
            name: e.name,
            industry: e.industry,
            description: e.description,
        }
    }
}

// ─── Knowledge Context for HUD ────────────────────────────────────────────

#[derive(Serialize, Clone)]
struct ContextResult {
    entity_type: String,
    name: String,
    relevance: f64,
    snippet: String,
}

#[derive(Serialize, Clone)]
struct StoryResult {
    id: String,
    title: String,
    tags: Option<String>,
    usage_count: i32,
    relevance: f64,
}

#[tauri::command]
fn search_knowledge_context(query: String, state: State<'_, AppState>) -> Result<Vec<ContextResult>, String> {
    let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    
    let searcher = knowledge::search::KnowledgeSearcher::new(&*graph);
    let results = searcher.context_search(&query);
    
    // Return top 5 results with relevance > 0.5
    let filtered: Vec<ContextResult> = results
        .into_iter()
        .filter(|r| r.relevance_score > 0.5)
        .take(5)
        .map(|r| ContextResult {
            entity_type: r.entity_type,
            name: r.name,
            relevance: r.relevance_score,
            snippet: r.snippet,
        })
        .collect();
    
    Ok(filtered)
}

#[tauri::command]
fn find_relevant_stories(context: String, state: State<'_, AppState>) -> Result<Vec<StoryResult>, String> {
    let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();
    
    let searcher = knowledge::search::KnowledgeSearcher::new(&*graph);
    let results = searcher.context_search(&context);
    
    // Filter to STAR stories and return top 3 with highest relevance
    let stories: Vec<StoryResult> = results
        .into_iter()
        .filter(|r| r.entity_type == "star_story")
        .take(3)
        .map(|r| StoryResult {
            id: r.entity_id,
            title: r.name,
            tags: None, // Will be enriched if we query the graph
            usage_count: 0,
            relevance: r.relevance_score,
        })
        .collect();
    
    // Enrich with usage_count and tags from graph
    let enriched: Vec<StoryResult> = stories
        .into_iter()
        .map(|mut s| {
            if let Ok(Some(story)) = graph.get_star_story(&s.id) {
                s.tags = story.tags;
                s.usage_count = story.usage_count;
            }
            s
        })
        .collect();
    
    Ok(enriched)
}

#[derive(Serialize)]
struct GraphData {
    skills: Vec<SkillRecord>,
    experiences: Vec<ExperienceWithSkills>,
    star_stories: Vec<StarStoryRecord>,
    companies: Vec<CompanyRecord>,
    edges: Vec<EdgeRecord>,
}

#[tauri::command]
fn get_graph_data(state: State<'_, AppState>) -> Result<GraphData, String> {
    let gp_lock = state.graph_provider.lock().map_err(|e| e.to_string())?;
    let provider = gp_lock.as_ref().ok_or_else(|| "Knowledge graph not initialized".to_string())?;
    let graph = provider.graph();

    let skills = graph.list_skills().map_err(|e| e.to_string())?
        .into_iter().map(SkillRecord::from).collect();

    let experiences_raw = graph.list_experiences().map_err(|e| e.to_string())?;
    let mut experiences = Vec::new();
    for exp in experiences_raw {
        let edges = graph.get_edges_for_entity(&exp.id, ghostai_pilot::knowledge::graph::EntityType::Experience)
            .unwrap_or_default();
        let mut skill_names = Vec::new();
        for edge in &edges {
            if edge.source_type == "skill" {
                if let Ok(Some(s)) = graph.get_skill(&edge.source_id) { skill_names.push(s.name); }
            } else if edge.target_type == "skill" {
                if let Ok(Some(s)) = graph.get_skill(&edge.target_id) { skill_names.push(s.name); }
            }
        }
        experiences.push(ExperienceWithSkills {
            id: exp.id, company: exp.company, role: exp.role,
            start_date: exp.start_date, end_date: exp.end_date,
            description: exp.description, highlights: exp.highlights,
            skills: skill_names,
        });
    }

    let star_stories = graph.list_star_stories().map_err(|e| e.to_string())?
        .into_iter().map(StarStoryRecord::from).collect();

    let companies = graph.list_companies().map_err(|e| e.to_string())?
        .into_iter().map(CompanyRecord::from).collect();

    let edges = graph.list_all_edges().map_err(|e| e.to_string())?
        .into_iter().map(EdgeRecord::from).collect();

    Ok(GraphData { skills, experiences, star_stories, companies, edges })
}

fn run_migration_on_startup(knowledge_base_path: &str) {
    let db_path = std::path::Path::new("pelendur.db");
    let yaml_path = std::path::PathBuf::from(knowledge_base_path)
        .join("personal")
        .join("profile.yaml");

    if !yaml_path.exists() {
        println!("  ℹ️  profile.yaml not found at {:?} — skipping migration", yaml_path);
        return;
    }

    let graph = match KnowledgeGraph::open(db_path) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("  ❌ Failed to open SQLite DB for migration: {}", e);
            return;
        }
    };

    let previous_ts = migration::last_migration_timestamp(&graph);
    match migration::migrate_profile_yaml(&yaml_path, &graph) {
        Ok(result) => {
            println!("  ✅ Migration complete:");
            println!("     Skills inserted: {}", result.skills_inserted);
            println!("     Stories inserted: {}", result.stories_inserted);
            println!("     Edges created: {}", result.edges_created);
            println!("     Weaknesses: {}", result.weaknesses_inserted);
            println!("     Achievements: {}", result.achievements_inserted);
            println!("     Preferences: {}", result.preferences_inserted);
            if result.skipped_existing > 0 {
                println!("     Skipped (existing): {}", result.skipped_existing);
            }
            if let Some(ts) = previous_ts {
                println!("     Previous migration: {}", ts);
            }
        }
        Err(e) => {
            eprintln!("  ❌ Migration failed: {}", e);
        }
    }
}

// ─── Practice Mode ───────────────────────────────────────────────────────

#[tauri::command]
async fn generate_practice_questions(
    state: State<'_, AppState>,
    mode: String,
    company_name: Option<String>,
) -> Result<Vec<serde_json::Value>, String> {
    let config = &state.config;
    
    // Get profile summary from knowledge manager
    let profile_summary = {
        let km = state.knowledge_manager.lock().map_err(|e| e.to_string())?;
        if let Some(profile) = &km.personal_profile {
            let all_skills: Vec<String> = profile.skills.dominados.iter()
                .chain(profile.skills.intermedios.iter())
                .map(|s| s.nombre.clone())
                .collect();
            format!(
                "Name: {}\nTitle: {}\nSkills: {}\nExperience: {}",
                profile.nombre,
                profile.rol_actual,
                all_skills.join(", "),
                profile.experiencia,
            )
        } else {
            "Unknown professional".to_string()
        }
    };
    
    let questions = ghostai_pilot::knowledge::practice::PracticeEngine::generate_questions(
        &mode,
        &profile_summary,
        company_name.as_deref(),
        config,
    ).await.map_err(|e| e.to_string())?;
    
    questions.into_iter()
        .map(|q| serde_json::to_value(q).map_err(|e| e.to_string()))
        .collect()
}

#[tauri::command]
async fn analyze_practice_answer(
    state: State<'_, AppState>,
    question: String,
    answer: String,
    mode: String,
) -> Result<serde_json::Value, String> {
    let config = &state.config;
    
    let feedback = ghostai_pilot::knowledge::practice::PracticeEngine::analyze_answer(
        &question,
        &answer,
        &mode,
        config,
    ).await.map_err(|e| e.to_string())?;
    
    serde_json::to_value(feedback).map_err(|e| e.to_string())
}

fn main() {
    dotenvy::from_filename(".env").ok();
    let config = config::Config::from_env().expect("Failed config");
    let mut km = knowledge::personal::KnowledgeManager::new("knowledge");
    let _ = km.load_personal_profile();
    let knowledge_manager = Arc::new(Mutex::new(km));

    run_migration_on_startup("knowledge");

    let graph_provider = match GraphKnowledgeProvider::open(std::path::Path::new("pelendur.db")) {
        Ok(gp) => {
            println!("  ✅ Knowledge graph initialized");
            Some(gp)
        }
        Err(e) => {
            eprintln!("  ⚠️  Knowledge graph init failed: {}", e);
            None
        }
    };

    tauri::Builder::default()
        .manage(AppState {
            config,
            knowledge_manager,
            graph_provider: Arc::new(Mutex::new(graph_provider)),
            is_locked: Arc::new(Mutex::new(false)),
            conversation: Arc::new(Mutex::new(Vec::new())),
            active_streams: Arc::new(Mutex::new(Vec::new())),
        })
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            if let Ok(shortcut) = "Ctrl+Alt+L".parse::<Shortcut>() {
                let _ = app.global_shortcut().register(shortcut);
            }
            Ok(())
        })
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new()
            .with_handler(|app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    let app_state = app.state::<AppState>();
                    let is_locked_lock = app_state.is_locked.clone();
                    let window = app.get_webview_window("main").unwrap();
                    let mut is_locked = is_locked_lock.lock().unwrap();
                    *is_locked = !*is_locked;
                    println!("  🔐 Lock: {}", *is_locked);
                    let _ = window.set_ignore_cursor_events(*is_locked);
                    let _ = window.emit("lock-state-changed", *is_locked);
                }
            })
            .build())
        .invoke_handler(tauri::generate_handler![start_capture, set_lock_state, clear_feed, regenerate, get_audio_processes, get_audio_devices, open_profile_window, get_knowledge_graph_stats, search_knowledge, search_knowledge_fuzzy, search_knowledge_enhanced, search_knowledge_semantic, analyze_meeting, parse_cv, confirm_cv_import, open_cv_dialog, create_star_story, update_star_story, delete_star_story, get_star_stories, coach_star_story, list_experiences_with_skills, list_skills, create_skill, update_skill, delete_skill, create_experience, update_experience, delete_experience, add_edge, remove_edge, list_edges_for_entity, get_graph_data, generate_practice_questions, analyze_practice_answer, search_knowledge_context, find_relevant_stories])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
