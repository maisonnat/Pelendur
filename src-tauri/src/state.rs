use ghostai_pilot::{config, knowledge, llm};
use ghostai_pilot::conversation_memory::ConversationMemory;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as TokioMutex;

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

/// Stores the active interview session metadata.
pub struct InterviewSession {
    pub company: String,
    pub company_context: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub turn_count: usize,
}

pub struct AppState {
    pub config: config::Config,
    pub knowledge_manager: Arc<Mutex<knowledge::personal::KnowledgeManager>>,
    pub graph_provider: Arc<TokioMutex<Option<ghostai_pilot::knowledge::graph::GraphKnowledgeProvider>>>,
    pub is_locked: Arc<Mutex<bool>>,
    pub is_minimal: Arc<Mutex<bool>>,
    pub conversation: Arc<Mutex<Vec<llm::ChatMessage>>>,
    pub active_streams: Arc<Mutex<Vec<StreamWrapper>>>,
    pub interview_session: Arc<Mutex<Option<InterviewSession>>>,
    pub memory: Arc<Mutex<ConversationMemory>>,
    #[cfg(feature = "parakeet")]
    pub parakeet_model: Arc<Mutex<Option<ghostai_pilot::parakeet::ParakeetModel>>>,
}

#[derive(Serialize, Clone)]
pub struct AudioLevelPayload {
    pub rms: f32,
    pub peak: f32,
    pub waveform: Vec<f32>,
    pub mode: String,
    pub sample_rate: u32,
}

#[derive(Serialize, Clone)]
pub struct AudioDevice {
    pub index: usize,
    pub name: String,
    pub label: String,
}