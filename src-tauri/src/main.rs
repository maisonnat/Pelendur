// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;
mod types;

use ghostai_pilot::knowledge::graph::{GraphKnowledgeProvider, KnowledgeGraph};
use ghostai_pilot::knowledge::migration;
use ghostai_pilot::{config, knowledge};
use state::AppState;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as TokioMutex;
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

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

fn main() {
    // Set CDP debugging port for WebView2 (needed when launched from WSL where env vars don't propagate)
    #[cfg(target_os = "windows")]
    {
        std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", "--remote-debugging-port=9224");
    }
    
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

    let memory = Arc::new(Mutex::new(
        ghostai_pilot::conversation_memory::ConversationMemory::new(
            config.engram_base_url.clone(),
            "pelendur",
            20,
        )
    ));

    // Check if Ollama is running locally (TCP port check — no extra deps)
    let ollama_available = Arc::new(Mutex::new(false));
    {
        let ollama_avail = ollama_available.clone();
        std::thread::spawn(move || {
            match std::net::TcpStream::connect_timeout(
                &"127.0.0.1:11434".parse().unwrap(),
                std::time::Duration::from_secs(2),
            ) {
                Ok(_) => {
                    if let Ok(mut avail) = ollama_avail.lock() {
                        *avail = true;
                        println!("  ✅ Ollama detected at localhost:11434");
                    }
                }
                Err(_) => {
                    println!("  ⚠️  Ollama not detected (expected if running cloud LLM)");
                }
            }
        });
    }

    // Initialize global Parakeet STT model
    #[cfg(feature = "parakeet")]
    {
        let model_dir = std::path::Path::new(&config.parakeet_model_dir);
        if model_dir.exists() {
            match ghostai_pilot::stt::init_parakeet_model(model_dir, true) {
                Ok(()) => println!("  ✅ Parakeet STT model loaded"),
                Err(e) => eprintln!("  ⚠️  Parakeet init failed: {}", e),
            }
        } else {
            eprintln!("  ⚠️  Parakeet model dir not found: {:?}", model_dir);
        }
    }

    // Initialize global whisper-rs STT model (used when parakeet feature is off)
    #[cfg(not(feature = "parakeet"))]
    {
        let model_path = &config.whisper_model_path;
        if std::path::Path::new(model_path).exists() {
            match ghostai_pilot::stt::init_whisper_rs(model_path) {
                Ok(()) => println!("  ✅ whisper-rs model loaded"),
                Err(e) => eprintln!("  ⚠️  whisper-rs init failed: {}", e),
            }
        } else {
            eprintln!("  ⚠️  whisper-rs model not found: {:?}", model_path);
            eprintln!("       Download: cd models && download-ggml-model.bat base.en");
        }
    }

    tauri::Builder::default()
        .manage(AppState {
            config,
            knowledge_manager,
            graph_provider: Arc::new(TokioMutex::new(graph_provider)),
            is_locked: Arc::new(Mutex::new(false)),
            is_minimal: Arc::new(Mutex::new(false)),
            conversation: Arc::new(Mutex::new(Vec::new())),
            active_streams: Arc::new(Mutex::new(Vec::new())),
            interview_session: Arc::new(Mutex::new(None)),
            memory,
            ollama_available,
            #[cfg(feature = "parakeet")]
            parakeet_model: Arc::new(Mutex::new(None)),
            #[cfg(feature = "testing")]
            test_metrics: Arc::new(Mutex::new(state::TestMetrics::default())),
        })
        .setup(|app| {
            let _window = app.get_webview_window("main").unwrap();

            // Emit system status to HUD
            let _ = app.emit("system-status", serde_json::json!({
                "stt": "ready",
                "llm": "ready",
                "kg": "ready",
            }));

            if let Ok(shortcut) = "Ctrl+Alt+L".parse::<Shortcut>() {
                let _ = app.global_shortcut().register(shortcut);
            }
            if let Ok(shortcut) = "Ctrl+Shift+Q".parse::<Shortcut>() {
                let _ = app.global_shortcut().register(shortcut);
            }
            Ok(())
        })
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new()
            .with_handler(|app, shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    if shortcut.to_string() == "Ctrl+Shift+Q" {
                        println!("  🚪 Quitting...");
                        app.exit(0);
                        return;
                    }
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
        .invoke_handler(tauri::generate_handler![
            // Audio
            commands::audio::get_audio_processes,
            commands::audio::get_audio_devices,
            commands::audio::start_capture,
            // UI
            commands::ui::set_lock_state,
            commands::ui::set_minimal_mode,
            commands::ui::clear_feed,
            commands::ui::regenerate,
            commands::ui::open_profile_window,
            commands::ui::close_app,
            // Knowledge search
            commands::knowledge::get_knowledge_graph_stats,
            commands::knowledge::search_knowledge,
            commands::knowledge::search_knowledge_fuzzy,
            commands::knowledge::search_knowledge_enhanced,
            commands::knowledge::search_knowledge_semantic,
            commands::knowledge::analyze_meeting,
            commands::knowledge::search_knowledge_context,
            commands::knowledge::find_relevant_stories,
            commands::knowledge::match_star_stories,
            commands::knowledge::match_star_stories_by_tags,
            commands::knowledge::record_star_story_usage,
            // Graph CRUD
            commands::graph::list_skills,
            commands::graph::create_skill,
            commands::graph::update_skill,
            commands::graph::delete_skill,
            commands::graph::list_experiences_with_skills,
            commands::graph::create_experience,
            commands::graph::update_experience,
            commands::graph::delete_experience,
            commands::graph::create_star_story,
            commands::graph::update_star_story,
            commands::graph::delete_star_story,
            commands::graph::get_star_stories,
            commands::graph::coach_star_story,
            commands::graph::add_edge,
            commands::graph::remove_edge,
            commands::graph::list_edges_for_entity,
            commands::graph::get_graph_data,
            // Interview Mode
            commands::interview::start_interview,
            commands::interview::end_interview,
            commands::interview::get_interview_state,
            commands::interview::list_company_research,
            // Company
            commands::company::list_companies,
            commands::company::create_company,
            commands::company::update_company,
            commands::company::delete_company,
            commands::company::load_company_research,
            commands::company::get_company_research_context,
            commands::company::refresh_company_research,
            // CV
            commands::cv::parse_cv,
            commands::cv::confirm_cv_import,
            commands::cv::open_cv_dialog,
            // Practice
            commands::practice::generate_practice_questions,
            commands::practice::analyze_practice_answer,
            // Company research
            commands::company::research_company,
            commands::company::list_unresearched_companies,
            commands::company::research_all_companies,
            #[cfg(feature = "testing")]
            commands::testing::get_test_metrics,
            #[cfg(feature = "testing")]
            commands::testing::inject_test_audio,
            #[cfg(feature = "testing")]
            commands::testing::get_hud_state,
            #[cfg(feature = "testing")]
            commands::testing::simulate_keyboard,
            #[cfg(feature = "testing")]
            commands::testing::set_mode,
            #[cfg(feature = "testing")]
            commands::testing::reset_metrics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}