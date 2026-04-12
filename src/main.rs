mod audio;
mod config;
mod llm;
mod loopback;
mod stt;
mod vad;

use anyhow::Result;
use chrono::Local;
use config::Config;
use llm::ChatMessage;
use std::sync::mpsc;
use tracing::{debug, info, warn, error};
use tracing_subscriber::EnvFilter;

const SYSTEM_PROMPT: &str = r#"You are a real-time meeting assistant. 
You receive transcribed audio from a conversation and provide concise, helpful responses.
IMPORTANT: Always respond in the SAME LANGUAGE as the transcribed text you receive.
Be brief — 2-3 sentences max. Focus on being useful.
If someone is asking a question, answer it directly.
If someone is making a statement, provide a relevant follow-up or insight.
If the transcription seems unclear or garbled, say so briefly."#;

/// Capture mode — either single device or dual (mic + app loopback)
enum CaptureMode {
    /// Single audio source (microphone or system audio)
    Single(audio::Device),
    /// Dual: microphone + per-application loopback
    Dual(audio::Device, loopback::real::AudioProcess),
}

/// Prompt user for capture mode and device selection.
fn select_capture_mode() -> Result<CaptureMode> {
    println!("  How do you want to capture audio?");
    println!();
    println!("    [1] Single device (microphone or system audio)");
    println!("    [2] Meeting Mode — Mic + App Loopback (Zoom, Chrome, etc.)");
    println!();
    print!("  Select [1-2]: ");
    use std::io::Write;
    std::io::stdout().flush().ok();

    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok();
    let input = input.trim();

    if input == "2" {
        // Meeting Mode: select mic + app
        println!();
        println!("  Step 1: Select your microphone");
        println!();
        let mic_device = audio::select_device_interactive()?;

        println!("  Step 2: Select the app to capture");
        println!();
        let app_process = loopback::real::select_audio_process()?;

        Ok(CaptureMode::Dual(mic_device, app_process))
    } else {
        // Single mode: select device
        let device = audio::select_device_interactive()?;
        Ok(CaptureMode::Single(device))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
                    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
        )
        .init();

    // Load config
    let config = Config::from_env()?;

    // Show STT provider
    let stt_label = match config.stt_provider {
        config::SttProvider::Groq => format!("Groq Whisper ({})", config.groq_stt_model),
        config::SttProvider::Local => format!("whisper.cpp ({})", config.whisper_model_path),
    };

    println!("┌─────────────────────────────────────────────┐");
    println!("│           GhostAI Audio Pilot               │");
    println!("│   System Audio → STT → LLM Pipeline         │");
    println!("└─────────────────────────────────────────────┘");
    println!();
    println!("  STT:  {}", stt_label);
    println!("  LLM:  {} ({})", config.openai_model, config.openai_base_url);
    println!();

    // Interactive capture mode selection
    let capture_mode = select_capture_mode()?;

    println!();
    println!("  Starting audio capture...");
    println!("  Speak or play audio. Press Ctrl+C to stop.");
    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Start audio capture(s) and create unified channel
    let audio_rx: mpsc::Receiver<audio::AudioChunk> = match capture_mode {
        CaptureMode::Single(device) => {
            // Simple: single capture, pass through directly
            audio::start_capture(device)?
        }
        CaptureMode::Dual(mic_device, app_process) => {
            // Dual: merge mic + loopback into a single channel
            info!(
                "Starting dual capture: mic + {} (PID {})",
                app_process.name, app_process.pid
            );

            let mic_rx = audio::start_capture(mic_device)?;
            let loop_rx =
                loopback::real::start_loopback_capture(app_process.pid, true)?;

            // Merge both into a single channel
            let (merged_tx, merged_rx) = mpsc::channel();

            // Forward mic audio
            let mic_tx = merged_tx.clone();
            std::thread::Builder::new()
                .name("mic-forward".to_string())
                .spawn(move || {
                    while let Ok(chunk) = mic_rx.recv() {
                        if mic_tx.send(chunk).is_err() {
                            break;
                        }
                    }
                })?;

            // Forward loopback audio
            let loop_tx = merged_tx;
            std::thread::Builder::new()
                .name("loop-forward".to_string())
                .spawn(move || {
                    while let Ok(chunk) = loop_rx.recv() {
                        if loop_tx.send(chunk).is_err() {
                            break;
                        }
                    }
                })?;

            merged_rx
        }
    };

    // Initialize VAD
    let mut vad_detector = vad::VadDetector::default_config();

    // Conversation history for context
    let mut conversation: Vec<ChatMessage> = vec![
        ChatMessage {
            role: "system".to_string(),
            content: SYSTEM_PROMPT.to_string(),
        },
    ];

    // Accumulate audio while speech is detected
    let mut speech_buffer: Vec<f32> = Vec::with_capacity(16000 * 10); // Up to 10 seconds
    let mut is_capturing = false;
    let mut chunk_count: usize = 0;

    // Main processing loop
    loop {
        // Receive audio chunk (blocking, 1 second chunks)
        let chunk = match audio_rx.recv() {
            Ok(c) => c,
            Err(_) => {
                warn!("Audio channel closed");
                break;
            }
        };

        chunk_count += 1;

        // Run VAD on this chunk
        let vad_event = vad_detector.process(&chunk.samples);

        match vad_event {
            vad::VadEvent::SpeechStart => {
                is_capturing = true;
                speech_buffer.clear();
                // Include the current chunk (it triggered speech detection)
                speech_buffer.extend_from_slice(&chunk.samples);

                let timestamp = Local::now().format("%H:%M:%S");
                print!("[{}] 🎤 ", timestamp);
                use std::io::Write;
                std::io::stdout().flush().ok();
            }
            vad::VadEvent::SpeechEnd { duration_chunks: _ } => {
                if is_capturing && !speech_buffer.is_empty() {
                    is_capturing = false;

                    println!("(processing...)");

                    // 1. Encode accumulated audio to WAV
                    let wav_bytes = stt::pcm_to_wav(&speech_buffer, chunk.sample_rate);

                    // Skip very short audio (<0.5 seconds = ~8000 samples)
                    if speech_buffer.len() < 8000 {
                        debug!("Audio too short, skipping");
                        continue;
                    }

                    // 2. Transcribe with STT
                    let transcription = match stt::transcribe(&config, &wav_bytes).await {
                        Ok(text) => text,
                        Err(e) => {
                            error!("STT failed: {}", e);
                            continue;
                        }
                    };

                    if transcription.trim().is_empty() {
                        continue;
                    }

                    let timestamp = Local::now().format("%H:%M:%S");
                    println!("[{}] 📝 \"{}\"", timestamp, transcription);

                    // 3. Add to conversation and generate response
                    conversation.push(ChatMessage {
                        role: "user".to_string(),
                        content: transcription.clone(),
                    });

                    // Keep conversation history manageable (last 20 messages)
                    if conversation.len() > 21 {
                        // Keep system prompt + last 20
                        let mut trimmed = vec![conversation[0].clone()];
                        trimmed.extend(conversation[conversation.len() - 20..].to_vec());
                        conversation = trimmed;
                    }

                    let timestamp = Local::now().format("%H:%M:%S");
                    print!("[{}] 🤖 ", timestamp);

                    match llm::generate_response(&config, &conversation).await {
                        Ok(response) => {
                            println!("{}", response);
                            conversation.push(ChatMessage {
                                role: "assistant".to_string(),
                                content: response,
                            });
                        }
                        Err(e) => {
                            error!("LLM failed: {}", e);
                            println!("[error: {}]", e);
                        }
                    }

                    println!();
                    println!("---");
                }
            }
            vad::VadEvent::Silence => {
                if is_capturing {
                    // Accumulate audio while speaking
                    speech_buffer.extend_from_slice(&chunk.samples);
                }

                // Periodic status (every ~30 seconds)
                if !is_capturing && chunk_count % 30 == 0 {
                    let timestamp = Local::now().format("%H:%M:%S");
                    println!("[{}] 👂 Listening... ({}s captured)", timestamp, chunk_count);
                }
            }
        }
    }

    Ok(())
}
