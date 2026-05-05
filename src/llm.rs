use crate::config::Config;
use anyhow::{Context, Result};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    max_tokens: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize)]
struct ChatChoice {
    delta: Option<ChatDelta>,
}

#[derive(Deserialize)]
struct ChatDelta {
    content: Option<String>,
    reasoning: Option<String>,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoiceFull>,
}

#[derive(Deserialize)]
struct ChatChoiceFull {
    message: ChatMessage,
}

/// Generate a response using OpenAI-compatible API (streaming)
/// Returns the full response text after streaming completes
pub async fn generate_response_streaming(
    config: &Config,
    messages: &[ChatMessage],
) -> Result<String> {
    let url = format!(
        "{}/chat/completions",
        config.openai_base_url.trim_end_matches('/')
    );

    let request = ChatRequest {
        model: config.openai_model.clone(),
        messages: messages.to_vec(),
        stream: true,
        max_tokens: 500, // Keep responses concise for real-time use
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.openai_api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .context("Failed to send LLM request")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("LLM API error {}: {}", status, body);
    }

    let mut full_response = String::new();
    let mut stream = response.bytes_stream();
    let mut is_thinking = false;

    debug!("LLM streaming started, model: {}", config.openai_model);

    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(c) => c,
            Err(e) => {
                debug!("LLM stream error: {}", e);
                anyhow::bail!("LLM stream error: {}", e);
            }
        };
        let text = String::from_utf8_lossy(&chunk);
        debug!(
            "LLM chunk ({} bytes): {}",
            chunk.len(),
            text.lines().next().unwrap_or("")
        );

        // Parse SSE: "data: {json}\n\n" format
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line == "data: [DONE]" {
                continue;
            }

            if let Some(json_str) = line.strip_prefix("data: ") {
                match serde_json::from_str::<ChatChoice>(json_str) {
                    Ok(choice) => {
                        if let Some(delta) = choice.delta {
                            // Show reasoning (thinking) tokens — don't add to history
                            if let Some(reasoning) = delta.reasoning {
                                if !reasoning.is_empty() {
                                    if !is_thinking {
                                        is_thinking = true;
                                        print!("\x1b[90m[thinking] ");
                                        use std::io::Write;
                                        std::io::stdout().flush().ok();
                                    }
                                    print!("\x1b[90m{}\x1b[0m", reasoning);
                                    use std::io::Write;
                                    std::io::stdout().flush().ok();
                                }
                            }
                            // Show content tokens — this IS the response
                            if let Some(content) = delta.content {
                                if !content.is_empty() {
                                    if is_thinking {
                                        is_thinking = false;
                                        println!("\x1b[90m[/thinking]\x1b[0m");
                                    }
                                    print!("{}", content);
                                    use std::io::Write;
                                    std::io::stdout().flush().ok();
                                    full_response.push_str(&content);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        debug!("LLM JSON parse error: {} — raw: {}", e, json_str);
                    }
                }
            }
        }
    }

    if is_thinking {
        println!("\x1b[90m[/thinking]\x1b[0m");
    }

    debug!(
        "LLM streaming done, response length: {}",
        full_response.len()
    );

    println!(); // Newline after streaming
    Ok(full_response)
}

/// Generate a response using OpenAI-compatible API (non-streaming, simpler)
pub async fn generate_response(config: &Config, messages: &[ChatMessage]) -> Result<String> {
    generate_response_with_options(config, messages, 500).await
}

/// Generate a response with a custom `max_tokens` limit.
/// Use for long-form outputs (CV parsing, reports) that exceed the default 500 tokens.
pub async fn generate_response_with_options(
    config: &Config,
    messages: &[ChatMessage],
    max_tokens: u32,
) -> Result<String> {
    let url = format!(
        "{}/chat/completions",
        config.openai_base_url.trim_end_matches('/')
    );

    let request = ChatRequest {
        model: config.openai_model.clone(),
        messages: messages.to_vec(),
        stream: false,
        max_tokens,
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.openai_api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .context("Failed to send LLM request")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("LLM API error {}: {}", status, body);
    }

    let body_text = response.text().await.unwrap_or_default();

    let result: ChatResponse =
        serde_json::from_str(&body_text).context("Failed to parse LLM response")?;

    let content = result
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default();

    Ok(content)
}
