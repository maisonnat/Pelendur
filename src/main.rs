mod audio;
mod audio_config;
mod config;
mod conversation_memory;
mod knowledge;
mod llm;
mod loopback;
#[cfg(feature = "linux_audio")]
mod linux_audio;
mod stt;
mod vad;
#[cfg(feature = "parakeet")]
mod parakeet;

#[cfg(feature = "parakeet")]
mod parakeet_inference {
    use std::sync::mpsc;
    use tokio::sync::{oneshot, broadcast};
    use anyhow::Result;

    pub enum InferenceRequest {
        /// Partial inference: fire-and-forget, result sent to broadcast channel
        Partial {
            samples: Vec<f32>,
        },
        /// Final inference: response expected via oneshot
        Final {
            samples: Vec<f32>,
            response_tx: oneshot::Sender<Result<String>>,
        },
    }

    pub type InferenceSender = mpsc::Sender<InferenceRequest>;
    pub type PartialReceiver = broadcast::Receiver<String>;

    pub fn spawn_inference_thread(model: crate::parakeet::ParakeetModel) -> (InferenceSender, broadcast::Sender<String>) {
        let (tx, rx) = mpsc::channel::<InferenceRequest>();
        let (partial_tx, _) = broadcast::channel::<String>(16);
        let partial_tx_clone = partial_tx.clone();

        std::thread::Builder::new()
            .name("parakeet-inference".to_string())
            .spawn(move || {
                let mut model = model;
                tracing::info!("Parakeet inference thread started");
                while let Ok(req) = rx.recv() {
                    match req {
                        InferenceRequest::Partial { samples } => {
                            if let Ok(text) = crate::stt::transcribe_parakeet_sync(&mut model, samples) {
                                if !text.trim().is_empty() {
                                    let _ = partial_tx_clone.send(text);
                                }
                            }
                        }
                        InferenceRequest::Final { samples, response_tx } => {
                            let result = crate::stt::transcribe_parakeet_sync(&mut model, samples);
                            let _ = response_tx.send(result);
                        }
                    }
                }
                tracing::info!("Parakeet inference thread stopped");
            })
            .expect("Failed to spawn parakeet inference thread");

        (tx, partial_tx)
    }
}

#[cfg(feature = "parakeet")]
use parakeet::{ParakeetEngine, ParakeetModel};

use anyhow::Result;
use audio_config::AudioStrategy;
use chrono::Local;
use config::Config;
use knowledge::personal::KnowledgeManager;
use llm::ChatMessage;
use tokio::sync::mpsc;
use tracing::{debug, info, warn, error};
use tracing_subscriber::EnvFilter;

/// Capture mode — either single device or dual (mic + system audio)
enum CaptureMode {
    Single(audio::Device),
    Dual(audio::Device, Option<u32>),
}

/// Simple linear resampling: convert f32 samples from `source_rate` to 16000 Hz.
/// Parakeet's nemo128 preprocessor expects 16 kHz input.
fn resample_to_16k(samples: &[f32], source_rate: u32) -> Vec<f32> {
    const TARGET_RATE: u32 = 16000;
    if source_rate == TARGET_RATE {
        return samples.to_vec();
    }
    let ratio = TARGET_RATE as f64 / source_rate as f64;
    let output_len = (samples.len() as f64 * ratio) as usize;
    let mut output = Vec::with_capacity(output_len);
    for i in 0..output_len {
        let src_pos = i as f64 / ratio;
        let idx = src_pos as usize;
        let frac = src_pos - idx as f64;
        let s0 = samples[idx];
        let s1 = samples.get(idx + 1).copied().unwrap_or(s0);
        output.push(s0 + (s1 - s0) * frac as f32);
    }
    output
}

fn select_strategy_if_needed(input: &str) -> Result<Option<Box<dyn AudioStrategy>>> {
    if input == "2" {
        let strategy = audio_config::detect_strategy()?;
        Ok(Some(strategy))
    } else {
        Ok(None)
    }
}

fn select_capture_mode() -> Result<CaptureMode> {
    println!("  How do you want to capture audio?");
    println!();
    println!("    [1] Single device (microphone or system audio)");
    println!("    [2] Meeting Mode — Mic + System Audio");
    println!();
    print!("  Select [1-2]: ");
    use std::io::Write;
    std::io::stdout().flush().ok();

    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok();
    let input = input.trim().to_string();

    let strategy = select_strategy_if_needed(&input)?;

    if input == "2" {
        let strategy = strategy.unwrap();
        println!();
        println!("  Step 1: Select your microphone");
        println!();
        let mic_device = audio::select_device_interactive()?;

        println!("  Step 2: Select system audio source");
        println!("  {}", strategy.name());
        let sources = strategy.list_sources();
        if sources.is_empty() {
            anyhow::bail!("No system audio sources found");
        }
        for (i, src) in sources.iter().enumerate() {
            println!("    [{}] {}", i + 1, src.name);
        }
        println!();

        #[cfg(feature = "wasapi_loopback")]
        {
            let app_process = loopback::real::select_audio_process()?;
            println!("  ✓ Selected: {} (PID: {})", app_process.name, app_process.pid);
            return Ok(CaptureMode::Dual(mic_device, Some(app_process.pid)));
        }

        #[cfg(not(feature = "wasapi_loopback"))]
        {
            println!("  ✓ System audio capture via {}", strategy.name());
            return Ok(CaptureMode::Dual(mic_device, None));
        }

        #[allow(unreachable_code)]
        Ok(CaptureMode::Dual(mic_device, None))
    } else {
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

    #[cfg(feature = "parakeet")]
    let (inference_tx, partial_tx) = {
        let engine = ParakeetEngine::new(
            std::path::PathBuf::from(&config.parakeet_model_dir)
        ).map_err(|e| anyhow::anyhow!("Failed to create Parakeet engine: {}", e))?;

        if !engine.is_model_ready() {
            tracing::warn!("Parakeet models not found. Downloading...");
            engine.ensure_models().await?;
            tracing::info!("Model download complete");
        }

        tracing::info!("Loading Parakeet model from {:?}", engine.model_dir());
        let model = ParakeetModel::new(engine.model_dir(), true)
            .map_err(|e| anyhow::anyhow!("Failed to load Parakeet model: {}", e))?;

        parakeet_inference::spawn_inference_thread(model)
    };

    #[cfg(feature = "parakeet")]
    {
        let mut partial_display_rx = partial_tx.subscribe();
        tokio::spawn(async move {
            while let Ok(text) = partial_display_rx.recv().await {
                let timestamp = Local::now().format("%H:%M:%S");
                println!("[{}] 📝 ~ \"{}\"", timestamp, text);
            }
        });
    }

    // Initialize Knowledge Manager
    let mut knowledge_manager = KnowledgeManager::new("knowledge");
    if let Err(e) = knowledge_manager.load_personal_profile() {
        warn!("Failed to load personal profile: {}", e);
    } else {
        info!("Personal profile loaded successfully");
    }

    let system_prompt = knowledge::personal::generate_system_prompt(&knowledge_manager);

    // Initialize Conversation Memory (Engram-backed cross-session memory)
    let mut memory = conversation_memory::ConversationMemory::new(
        &config.engram_base_url,
        "pelendur",
        5, // max 5 past memories into context
    );
    let session_title = format!("Interview Session {}", Local::now().format("%Y-%m-%d %H:%M"));
    if let Err(e) = memory.start_session(&session_title).await {
        warn!("Failed to start Engram memory session: {} — memory disabled", e);
    } else {
        info!("Engram memory session active");
    }

    // Show STT provider
    let stt_label = match config.stt_provider {
        config::SttProvider::Groq => format!("Groq Whisper ({})", config.groq_stt_model),
        config::SttProvider::Zai => "z.ai GLM-ASR-2512".to_string(),
        config::SttProvider::Local => {
            #[cfg(feature = "parakeet")]
            { "Parakeet ONNX (local)".to_string() }
            #[cfg(not(feature = "parakeet"))]
            { format!("whisper.cpp ({})", config.whisper_model_path) }
        }
    };

    println!("┌─────────────────────────────────────────────┐");
    println!("│               Pelendur                     │");
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
    println!();

    // Start audio capture(s) and create unified channel
    let (audio_tx, mut audio_rx) = mpsc::channel::<audio::AudioChunk>(100);
    
    // IMPORTANT: We must keep these streams alive throughout main
    let mut _active_streams = Vec::new();

    match capture_mode {
        CaptureMode::Single(device) => {
            let (rx, stream) = audio::start_capture(device)?;
            _active_streams.push(stream);
            let tx = audio_tx.clone();
            std::thread::Builder::new()
                .name("audio-forward".to_string())
                .spawn(move || {
                    while let Ok(chunk) = rx.recv() {
                        if tx.blocking_send(chunk).is_err() {
                            break;
                        }
                    }
                })?;
        }
        CaptureMode::Dual(mic_device, pid) => {
            let strategy = match pid {
                #[cfg(feature = "wasapi_loopback")]
                Some(p) => {
                    use audio_config::WindowsStrategy;
                    Box::new(WindowsStrategy::new().with_process(p)) as Box<dyn AudioStrategy>
                }
                _ => audio_config::detect_strategy()?,
            };
            info!("Starting system audio capture via {}", strategy.name());

            let loop_rx = strategy.start_system_capture()?;

            // Try microphone as supplementary source
            match audio::start_capture(mic_device) {
                Ok((mic_rx, mic_stream)) => {
                    info!("Dual capture: mic + loopback active");
                    _active_streams.push(mic_stream);
                    let mic_tx = audio_tx.clone();
                    std::thread::Builder::new()
                        .name("mic-forward".to_string())
                        .spawn(move || {
                            while let Ok(chunk) = mic_rx.recv() {
                                if mic_tx.blocking_send(chunk).is_err() {
                                    break;
                                }
                            }
                        })?;
                }
                Err(e) => {
                    warn!("Mic capture failed ({}), using loopback only", e);
                }
            }

            let loop_tx = audio_tx;
            std::thread::Builder::new()
                .name("loop-forward".to_string())
                .spawn(move || {
                    while let Ok(chunk) = loop_rx.recv() {
                        if loop_tx.blocking_send(chunk).is_err() {
                            break;
                        }
                    }
                })?;
        }
    }

    // Initialize VAD
    let mut vad_detector = vad::VadDetector::default_config();

    // Conversation history for context
    let mut conversation: Vec<ChatMessage> = vec![
        ChatMessage {
            role: "system".to_string(),
            content: system_prompt,
        },
    ];

    // Accumulate audio while speech is detected
    let mut speech_buffer: Vec<f32> = Vec::with_capacity(16000 * 10);
    let mut is_capturing = false;
    let mut chunk_count: usize = 0;
    let mut buffer_sample_rate: u32 = 16000; // track for resampling
    #[cfg(feature = "parakeet")]
    let mut last_partial_samples: usize = 0;

    println!("  👂 Listening... (Audio levels every ~2s)");
    println!("  Speak or play audio. Press Ctrl+C to stop.");
    println!();
    println!("─────────────────────────────────────────────");
    println!();

    // Main processing loop
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!("\n  Stopping Pelendur...");
                println!("  Saving interview session to knowledge base...");
                
                let mut session_content = String::from("# Interview Session Summary\n\n");
                for msg in &conversation {
                    if msg.role != "system" {
                        session_content.push_str(&format!("**{}:** {}\n\n", msg.role.to_uppercase(), msg.content));
                    }
                }
                
                knowledge_manager.save_to_all(&session_title, &session_content);

                // Save session summary to Engram
                let summary = format!("Interview session of {} with {} turns.", 
                    Local::now().format("%Y-%m-%d %H:%M"),
                    conversation.iter().filter(|m| m.role == "user").count());
                if let Err(e) = memory.end_session(&summary).await {
                    debug!("Failed to save session summary to Engram: {}", e);
                } else {
                    println!("  ✓ Session saved to Engram memory.");
                }

                println!("  ✓ Session saved. Goodbye!");
                break;
            }
            
            chunk = audio_rx.recv() => {
                let chunk = match chunk {
                    Some(c) => c,
                    None => {
                        warn!("Audio channel closed");
                        break;
                    }
                };

                chunk_count += 1;
                let vad_event = vad_detector.process(&chunk.samples);

                // Show audio level every 2 chunks (~2s) for debugging
                if chunk_count % 2 == 0 {
                    let rms: f32 = chunk.samples.iter().map(|s| s * s).sum::<f32>() / chunk.samples.len() as f32;
                    let rms_db = 20.0 * rms.max(1e-10).log10();
                    let timestamp = Local::now().format("%H:%M:%S");
                    println!("[{}] 👂 Audio level: {:.1} dB (threshold: -35 dB)", timestamp, rms_db);
                }

                match vad_event {
                    vad::VadEvent::SpeechStart => {
                        is_capturing = true;
                        speech_buffer.clear();
                        buffer_sample_rate = chunk.sample_rate;
                        speech_buffer.extend_from_slice(&chunk.samples);
                        let timestamp = Local::now().format("%H:%M:%S");
                        print!("[{}] 🎙 ", timestamp);
                        use std::io::Write;
                        std::io::stdout().flush().ok();
                    }
                    vad::VadEvent::SpeechEnd { duration_chunks: _ } => {
                        if is_capturing && !speech_buffer.is_empty() {
                            is_capturing = false;
                            println!("(processing...)");

                            if speech_buffer.len() < 8000 {
                                debug!("Audio too short, skipping");
                                continue;
                            }

                            #[cfg(feature = "parakeet")]
                            let transcription = if config.stt_provider == config::SttProvider::Local {
                                let resampled = resample_to_16k(&speech_buffer, buffer_sample_rate);
                                let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                                if inference_tx.send(parakeet_inference::InferenceRequest::Final {
                                    samples: resampled,
                                    response_tx: resp_tx,
                                }).is_err() {
                                    error!("Inference channel closed");
                                    continue;
                                }
                                match resp_rx.await {
                                    Ok(Ok(text)) => text,
                                    Ok(Err(e)) => {
                                        error!("Parakeet inference failed: {}", e);
                                        continue;
                                    }
                                    Err(_) => {
                                        error!("Inference response dropped");
                                        continue;
                                    }
                                }
                            } else {
                                let wav_bytes = match stt::pcm_to_wav(&speech_buffer, chunk.sample_rate) {
                                    Ok(bytes) => bytes,
                                    Err(e) => {
                                        error!("WAV encoding failed: {}", e);
                                        continue;
                                    }
                                };
                                match stt::transcribe(&config, &wav_bytes).await {
                                    Ok(text) => text,
                                    Err(e) => {
                                        error!("STT failed: {}", e);
                                        continue;
                                    }
                                }
                            };

                            #[cfg(not(feature = "parakeet"))]
                            let transcription = {
                                let wav_bytes = match stt::pcm_to_wav(&speech_buffer, chunk.sample_rate) {
                                    Ok(bytes) => bytes,
                                    Err(e) => {
                                        error!("WAV encoding failed: {}", e);
                                        continue;
                                    }
                                };
                                match stt::transcribe(&config, &wav_bytes).await {
                                    Ok(text) => text,
                                    Err(e) => {
                                        error!("STT failed: {}", e);
                                        continue;
                                    }
                                }
                            };

                            if transcription.trim().is_empty() {
                                continue;
                            }

                            let timestamp = Local::now().format("%H:%M:%S");
                            println!("[{}] 📝 \"{}\"", timestamp, transcription);

                            // Match Knowledge (The Brain)
                            let relevant_stories = if let Some(profile) = &knowledge_manager.personal_profile {
                                profile.find_relevant_stories(&transcription)
                            } else {
                                vec![]
                            };

                            let external_knowledge = knowledge_manager.search_all(&transcription);

                            // Build combined context from knowledge + Engram memory
                            let mut context_messages: Vec<ChatMessage> = Vec::new();

                            if !relevant_stories.is_empty() || !external_knowledge.is_empty() {
                                let mut context_msg = String::from("RELEVANT CONTEXT FOUND:\n");
                                for story in relevant_stories {
                                    context_msg.push_str(&format!("- STAR STORY [{}]: {} -> {}\n", 
                                        story.id, story.situacion, story.resultado));
                                }
                                for ext in external_knowledge {
                                    context_msg.push_str(&format!("- EXTERNAL: {}\n", ext));
                                }
                                context_messages.push(ChatMessage {
                                    role: "system".to_string(),
                                    content: context_msg,
                                });
                            }

                            // Load cross-session memory from Engram
                            if let Ok(Some(memory_context)) = memory.build_memory_context(&transcription).await {
                                context_messages.push(ChatMessage {
                                    role: "system".to_string(),
                                    content: memory_context,
                                });
                                info!("Injected cross-session memory context for: {}", &transcription[..transcription.len().min(60)]);
                            }

                            // Push all context messages, then the user message
                            for ctx_msg in context_messages {
                                conversation.push(ctx_msg);
                            }

                            conversation.push(ChatMessage {
                                role: "user".to_string(),
                                content: transcription.clone(),
                            });

                            if conversation.len() > 25 {
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
                                        content: response.clone(),
                                    });
                                    // Save Q&A turn to Engram (best-effort)
                                    if let Err(e) = memory.save_turn(&transcription, &response).await {
                                        debug!("Failed to save conversation turn to Engram: {}", e);
                                    }
                                }
                                Err(e) => {
                                    error!("LLM failed: {}", e);
                                    println!("[error: {}]", e);
                                }
                            }

                            println!();
                            println!("---");
                        }

                        #[cfg(feature = "parakeet")]
                        { last_partial_samples = 0; }
                    }
                    vad::VadEvent::Silence => {
                        if is_capturing {
                            speech_buffer.extend_from_slice(&chunk.samples);

                            #[cfg(feature = "parakeet")]
                            if config.stt_provider == config::SttProvider::Local {
                                let new_samples = speech_buffer.len().saturating_sub(last_partial_samples);
                                if new_samples >= 32000 && speech_buffer.len() >= 16000 {
                                    tracing::trace!("Sending partial inference ({} samples)", speech_buffer.len());
                                    let resampled = resample_to_16k(&speech_buffer, buffer_sample_rate);
                                    let _ = inference_tx.send(parakeet_inference::InferenceRequest::Partial {
                                        samples: resampled,
                                    });
                                    last_partial_samples = speech_buffer.len();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resample_identity() {
        // 16kHz → 16kHz should be passthrough
        let samples = vec![0.5f32; 16000];
        let result = resample_to_16k(&samples, 16000);
        assert_eq!(result.len(), 16000);
    }

    #[test]
    fn test_resample_48k_to_16k() {
        // 48000 → 16000 = 1/3 ratio
        let samples: Vec<f32> = (0..48000).map(|i| (i as f32 / 48000.0)).collect();
        let result = resample_to_16k(&samples, 48000);
        assert_eq!(result.len(), 16000);
        // First sample should be ~0
        assert!(result[0].abs() < 0.01);
        // Last sample should be close to 1.0
        assert!(*result.last().unwrap() > 0.9 || result.last().unwrap().abs() < 0.01);
    }

    #[test]
    fn test_resample_44100_to_16k() {
        // Loopback typical rate
        let samples = vec![0.3f32; 44100];
        let result = resample_to_16k(&samples, 44100);
        let expected_len = (44100.0 * 16000.0 / 44100.0) as usize;
        assert_eq!(result.len(), expected_len);
        // All values should be ~0.3
        for &s in &result {
            assert!((s - 0.3).abs() < 0.01);
        }
    }
}
