use anyhow::{Context, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum SttProvider {
    Groq,
    Local,
}

#[derive(Debug, Clone)]
pub struct Config {
    // STT provider selection
    pub stt_provider: SttProvider,

    // Groq STT
    pub groq_api_key: String,
    pub groq_stt_model: String,

    // Local whisper.cpp
    pub whisper_model_path: String,
    pub whisper_language: String,

    // LLM
    pub openai_api_key: String,
    pub openai_model: String,
    pub openai_base_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let stt_provider = match std::env::var("STT_PROVIDER")
            .unwrap_or_else(|_| "local".to_string())
            .to_lowercase()
            .as_str()
        {
            "groq" => SttProvider::Groq,
            _ => SttProvider::Local,
        };

        // Groq keys only required if using Groq
        let groq_api_key = if stt_provider == SttProvider::Groq {
            std::env::var("GROQ_API_KEY").context("GROQ_API_KEY not set in .env (required when STT_PROVIDER=groq)")?
        } else {
            String::new()
        };

        Ok(Self {
            stt_provider,
            groq_api_key,
            groq_stt_model: std::env::var("GROQ_STT_MODEL")
                .unwrap_or_else(|_| "whisper-large-v3-turbo".to_string()),
            whisper_model_path: std::env::var("WHISPER_MODEL_PATH")
                .unwrap_or_else(|_| "models/ggml-base.en.bin".to_string()),
            whisper_language: std::env::var("WHISPER_LANGUAGE")
                .unwrap_or_else(|_| "en".to_string()),
            openai_api_key: std::env::var("OPENAI_API_KEY")
                .unwrap_or_else(|_| "ollama".to_string()),
            openai_model: std::env::var("OPENAI_MODEL")
                .unwrap_or_else(|_| "qwen2.5:7b".to_string()),
            openai_base_url: std::env::var("OPENAI_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:11434/v1".to_string()),
        })
    }
}
