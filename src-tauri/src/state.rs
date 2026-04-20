use ghostai_pilot::{config, knowledge, llm};
use serde::Serialize;
use std::sync::{Arc, Mutex};

/// Wrapper to make cpal::Stream Send-safe.
/// cpal::Stream is already Send on most platforms but not Sync.
/// We only need to store it to keep it alive; we never access it after creation.
pub struct StreamWrapper(pub cpal::Stream);
unsafe impl Send for StreamWrapper {}
// SAFETY: StreamWrapper is never shared between threads — it's only stored
// in a Mutex<Vec<StreamWrapper>> to keep the audio streams alive.

#[derive(Serialize, Clone)]
pub struct TranscriptionPayload {
    pub text: String,
}

#[derive(Serialize, Clone)]
pub struct SuggestionPayload {
    pub text: String,
}

pub struct AppState {
    pub config: config::Config,
    pub knowledge_manager: Arc<Mutex<knowledge::personal::KnowledgeManager>>,
    pub graph_provider: Arc<Mutex<Option<ghostai_pilot::knowledge::graph::GraphKnowledgeProvider>>>,
    pub is_locked: Arc<Mutex<bool>>,
    pub conversation: Arc<Mutex<Vec<llm::ChatMessage>>>,
    pub active_streams: Arc<Mutex<Vec<StreamWrapper>>>,
}

#[derive(Serialize, Clone)]
pub struct AudioDevice {
    pub index: usize,
    pub name: String,
    pub label: String,
}
