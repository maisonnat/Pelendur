use anyhow::Result;
use ghostai_pilot::config::Config;
use ghostai_pilot::llm::{self, ChatMessage};

#[tokio::test]
async fn test_ollama_generates_spanish_interview_response() -> Result<()> {
    // Load config from .env (or defaults: Ollama at localhost:11434, qwen3:4b-instruct)
    let config = Config::from_env()?;

    println!(
        "Testing LLM connection to: {} with model: {}",
        config.openai_base_url, config.openai_model
    );

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: "Eres un asistente de entrevistas técnicas. Responde en español de forma útil, concisa y profesional.".to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: "¿Cómo responderías a 'Cuéntame sobre ti' en una entrevista técnica para un puesto de Rust developer?".to_string(),
        },
    ];

    let response = llm::generate_response(&config, &messages).await?;

    println!("=== LLM Response ===");
    println!("{}", response);
    println!("=== End ===");

    assert!(!response.is_empty(), "Response should not be empty");
    assert!(
        response.len() > 50,
        "Response should be substantial (>50 chars), got {} chars",
        response.len()
    );

    // Response should contain Spanish text relevant to interview context
    let has_spanish_chars = response.chars().any(|c| "áéíóúñÁÉÍÓÚÑ¿¡".contains(c));
    assert!(
        has_spanish_chars,
        "Response should contain Spanish characters"
    );

    // Should mention Rust, developer, or interview context
    let keywords = [
        "Rust",
        "desarrollador",
        "ingeniero",
        "experiencia",
        "técnica",
        "entrevista",
    ];
    let has_keyword = keywords.iter().any(|k| response.contains(k));
    assert!(
        has_keyword,
        "Response should contain at least one interview-relevant keyword. Got: {:.80}",
        response
    );

    println!(
        "\n✓ Test passed: Ollama qwen3:4b-instruct generates coherent Spanish interview responses"
    );

    Ok(())
}
