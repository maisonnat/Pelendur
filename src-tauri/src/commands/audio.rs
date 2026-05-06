use crate::state::{AppState, AudioDevice, AudioLevelPayload, StreamWrapper, TranscriptionPayload, SuggestionPayload};
use ghostai_pilot::{audio, knowledge, loopback, mixer, stt, vad};
use ghostai_pilot::llm::ChatMessage;
use tauri::{AppHandle, Manager, State, Emitter};
use cpal::traits::{DeviceTrait, HostTrait};
use futures_util::StreamExt;

// ── Audio Level Computation ──────────────────────────────────────────────

/// Compute RMS and peak level from f32 audio samples.
/// Samples are expected in [-1.0, 1.0] range.
fn compute_audio_levels(samples: &[f32]) -> (f32, f32) {
    if samples.is_empty() {
        return (0.0, 0.0);
    }
    let mut sum_sq = 0.0f64;
    let mut peak = 0.0f32;
    for &s in samples {
        let abs = s.abs();
        if abs > peak {
            peak = abs;
        }
        sum_sq += (abs as f64) * (abs as f64);
    }
    let rms = (sum_sq / samples.len() as f64).sqrt() as f32;
    (rms, peak)
}

/// Downsample audio samples to `target_points` for waveform display.
/// Uses simple linear decimation.
fn downsample_waveform(samples: &[f32], target_points: usize) -> Vec<f32> {
    if samples.is_empty() || target_points == 0 {
        return Vec::new();
    }
    if samples.len() <= target_points {
        return samples.to_vec();
    }
    let step = samples.len() / target_points;
    let mut result = Vec::with_capacity(target_points);
    for i in 0..target_points {
        let idx = i * step;
        // Take max absolute value in this window for peak waveform representation
        let end = (idx + step).min(samples.len());
        let window = &samples[idx..end];
        let max_val = window.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        result.push(max_val);
    }
    result
}

#[tauri::command]
pub fn get_audio_processes() -> Result<Vec<loopback::AudioProcess>, String> {
    Ok(loopback::list_audio_processes())
}

#[tauri::command]
pub fn get_audio_devices() -> Result<Vec<AudioDevice>, String> {
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
pub async fn start_capture(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    mode: Option<String>,
    device_index: Option<usize>,
) -> Result<(), String> {
    let config = state.config.clone();
    let km_lock = state.knowledge_manager.clone();
    let conversation_lock = state.conversation.clone();
    let streams_lock = state.active_streams.clone();
    let interview_session = state.interview_session.clone();
    let memory = state.memory.clone();

    {
        let mut streams = streams_lock.lock().map_err(|e| e.to_string())?;
        streams.clear();
    }

    // Determine capture source: "system" = WASAPI loopback, "mic" = cpal microphone, "dual" = both mixed 50/50
    let capture_mode = mode.unwrap_or_else(|| {
        // Default to system loopback on Windows, mic on Linux
        if cfg!(target_os = "windows") { "system".to_string() } else { "mic".to_string() }
    });

    let audio_rx = if capture_mode == "system" {
        // WASAPI loopback — captures all audio playing through the output device
        println!("  🔊 Starting WASAPI loopback capture (system audio)...");
        loopback::start_system_loopback_capture().map_err(|e| format!("Loopback failed: {}", e))?
    } else if capture_mode == "dual" {
        // Dual capture: WASAPI loopback + microphone, mixed 50/50
        println!("  🎙️ Starting dual capture (system audio + microphone mixed 50/50)...");

        let host = cpal::default_host();
        let devices: Vec<_> = host.input_devices().map_err(|e| e.to_string())?.collect();
        let device = if let Some(idx) = device_index {
            devices.get(idx).ok_or_else(|| "Invalid index".to_string())?.clone()
        } else {
            audio::find_microphone_device().map_err(|e| e.to_string())?
        };
        println!("  🎤 Mic: {:?}", device.name().unwrap_or_default());

        let (rx, mic_stream) = mixer::start_dual_capture(device)?;
        let mut streams = streams_lock.lock().map_err(|e| e.to_string())?;
        streams.push(StreamWrapper(mic_stream));
        rx
    } else {
        // Microphone capture via cpal
        let host = cpal::default_host();
        let devices: Vec<_> = host.input_devices().map_err(|e| e.to_string())?.collect();
        let device = if let Some(idx) = device_index {
            devices.get(idx).ok_or_else(|| "Invalid index".to_string())?.clone()
        } else {
            audio::find_microphone_device().map_err(|e| e.to_string())?
        };
        println!("  🎤 Captura mic: {:?}", device.name().unwrap_or_default());
        let (rx, stream) = audio::start_capture(device).map_err(|e| e.to_string())?;
        let mut streams = streams_lock.lock().map_err(|e| e.to_string())?;
        streams.push(StreamWrapper(stream));
        rx
    };

    println!("  ✅ Audio capture active (mode: {})", capture_mode);

    let capture_mode_clone = capture_mode.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut vad_detector = vad::VadDetector::default_config();

        // Waveform target points for the HUD display
        const WAVEFORM_POINTS: usize = 180;

        {
            let mut conversation = conversation_lock.lock().unwrap();
            if conversation.is_empty() {
                let km = km_lock.lock().unwrap();
                let system_prompt = knowledge::personal::generate_system_prompt(&km);
                conversation.push(ChatMessage { role: "system".to_string(), content: system_prompt });
            }
        }

        macro_rules! diag { ($($arg:tt)*) => {{
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("pelendur-pipe.log") {
                use std::io::Write;
                let _ = writeln!(f, $($arg)*);
            }
        }}; }

        let mut speech_buffer: Vec<f32> = Vec::with_capacity(16000 * 10);
        let mut is_capturing = false;
        // Track last partial transcription position (in samples @ chunk rate)
        let mut last_partial_pos: usize = 0;

        while let Ok(chunk) = audio_rx.recv() {
            let rms_val = (chunk.samples.iter().map(|s|s*s).sum::<f32>()/chunk.samples.len() as f32).sqrt();
            diag!("[DIAG] Chunk: {}samp {}Hz rms={:.4}", chunk.samples.len(), chunk.sample_rate, rms_val);
            // Audio level visualization
            if let Ok(mut diag_f) = std::fs::OpenOptions::new().create(true).append(true).open("pelendur-pipe.log") {
                use std::io::Write;
                let rms = (chunk.samples.iter().map(|s|s*s).sum::<f32>()/chunk.samples.len() as f32).sqrt();
                let _ = writeln!(diag_f, "[DIAG] Chunk: {}samp {}Hz rms={:.4}", 
                    chunk.samples.len(), chunk.sample_rate, rms);
            }
            // Audio level visualization ────────────────────────────────
            let (rms, peak) = compute_audio_levels(&chunk.samples);
            let waveform = downsample_waveform(&chunk.samples, WAVEFORM_POINTS);
            emit_to_window(&app_handle, "audio-level-update", AudioLevelPayload {
                rms,
                peak,
                waveform,
                mode: capture_mode_clone.clone(),
                sample_rate: chunk.sample_rate,
            });

            let vad_event = vad_detector.process(&chunk.samples);
            match vad_event {
                vad::VadEvent::SpeechStart => {
                    diag!("[DIAG] VAD SpeechStart (rms={:.4})", rms_val);
                    is_capturing = true;
                    speech_buffer.clear();
                    last_partial_pos = 0;
                    emit_to_window(&app_handle, "partial-transcription",
                        TranscriptionPayload { text: String::new() });
                    speech_buffer.extend_from_slice(&chunk.samples);
                }
                vad::VadEvent::SpeechEnd { .. } => {
                    diag!("[DIAG] VAD SpeechEnd (buf={}samp, {}Hz)", speech_buffer.len(), chunk.sample_rate);
                    if is_capturing && !speech_buffer.is_empty() {
                        is_capturing = false;
                        let wav_bytes = match stt::pcm_to_wav(&speech_buffer, chunk.sample_rate) {
                            Ok(bytes) => bytes,
                            Err(e) => {
                                eprintln!("WAV encoding failed: {}", e);
                                continue;
                            }
                        };
                        if speech_buffer.len() < 8000 {
                            diag!("[DIAG] Buffer <8k, skip");
                            continue;
                        }
                        // Limit buffer to 2 seconds for fast local inference
                        let max_samp = chunk.sample_rate as usize * 2;
                        if speech_buffer.len() > max_samp {
                            diag!("[DIAG] Trunc buf {} -> {}", speech_buffer.len(), max_samp);
                            speech_buffer.truncate(max_samp);
                        }

                        diag!("[DIAG] Calling STT... buf={}samp {}Hz", speech_buffer.len(), chunk.sample_rate);

                        // Run STT in a dedicated thread with 30s timeout
                        let (tx_stt, rx_stt) = std::sync::mpsc::channel();
                        let config_stt = config.clone();
                        let wav_stt = wav_bytes.clone();
                        std::thread::Builder::new()
                            .name("stt-worker".into())
                            .spawn(move || {
                                let result = stt::transcribe_local_sync(&config_stt, &wav_stt);
                                let _ = tx_stt.send(result);
                            })
                            .expect("Failed to spawn STT worker");

                        let stt_result = match rx_stt.recv_timeout(std::time::Duration::from_secs(30)) {
                            Ok(Ok(t)) => Some(t),
                            Ok(Err(e)) => { diag!("[DIAG] STT error: {}", e); None }
                            Err(_) => { diag!("[DIAG] STT timeout (30s)"); None }
                        };
                        if let Some(transcription) = stt_result {
                            if transcription.trim().is_empty() {
                                diag!("[DIAG] STT returned empty");
                                continue;
                            }
                            diag!("[DIAG] STT OK ({}chars): {}", transcription.len(), &transcription[..80.min(transcription.len())]);
                            println!("  📝 \"{}\"", transcription);

                            emit_to_window(&app_handle, "transcription-update", TranscriptionPayload { text: transcription.clone() });

                            let (relevant_stories, external_knowledge) = {
                                let km = km_lock.lock().unwrap();
                                let stories: Vec<_> = if let Some(profile) = &km.personal_profile {
                                    profile.find_relevant_stories(&transcription).into_iter().cloned().collect()
                                } else {
                                    vec![]
                                };
                                let ext = km.search_all(&transcription);
                                (stories, ext)
                            };

                            let mut conversation = conversation_lock.lock().unwrap();

                            const MAX_CONVERSATION_LEN: usize = 21;
                            if conversation.len() > MAX_CONVERSATION_LEN {
                                let excess = conversation.len() - (MAX_CONVERSATION_LEN - 1);
                                conversation.drain(1..1 + excess);
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

                            // Clone transcription for Engram before moving it
                            let transcription_for_engram = transcription.clone();
                            conversation.push(ChatMessage { role: "user".to_string(), content: transcription });

                            // --- Streaming LLM response to HUD ---
                            // Shows tokens progressively instead of waiting for full response
                            let url = format!("{}/chat/completions",
                                config.openai_base_url.trim_end_matches('/'));
                            let llm_request = serde_json::json!({
                                "model": config.openai_model,
                                "messages": &*conversation,
                                "stream": true,
                                "max_tokens": 500,
                            });

                            let llm_result: Result<String, String> = rt.block_on(async {
                                let client = reqwest::Client::builder()
                                    .timeout(std::time::Duration::from_secs(120))
                                    .build()
                                    .map_err(|e| format!("Client: {}", e))?;

                                let response = client
                                    .post(&url)
                                    .header("Authorization", format!("Bearer {}", config.openai_api_key))
                                    .header("Content-Type", "application/json")
                                    .json(&llm_request)
                                    .send()
                                    .await
                                    .map_err(|e| format!("Send: {}", e))?;

                                let mut full = String::new();
                                let mut stream = response.bytes_stream();
                                while let Some(chunk) = stream.next().await {
                                    let chunk = match chunk {
                                        Ok(c) => c,
                                        Err(e) => { eprintln!("  ⚠️ LLM stream: {}", e); break; }
                                    };
                                    let text = String::from_utf8_lossy(&chunk);
                                    for line in text.lines() {
                                        let line = line.trim();
                                        if line.is_empty() || line == "data: [DONE]" { continue; }
                                        if let Some(json_str) = line.strip_prefix("data: ") {
                                            if let Ok(choice) = serde_json::from_str::<serde_json::Value>(json_str) {
                                                if let Some(delta) = choice["choices"][0]["delta"]["content"].as_str() {
                                                    if !delta.is_empty() {
                                                        full.push_str(delta);
                                                        // Emit each token to HUD in real-time
                                                        let _ = emit_to_window(&app_handle, "suggestion-stream",
                                                            SuggestionPayload { text: delta.to_string() });
                                                        print!("{}", delta);
                                                        use std::io::Write;
                                                        std::io::stdout().flush().ok();
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                Ok::<String, String>(full)
                            });

                            match llm_result {
                                Ok(response) => {
                                    println!();
                                    println!("  🤖 IA: {}", response);
                                    emit_to_window(&app_handle, "suggestion-update",
                                        SuggestionPayload { text: response.clone() });
                                    conversation.push(ChatMessage {
                                        role: "assistant".to_string(),
                                        content: response.clone(),
                                    });

                                    // Store the turn to Engram if in interview mode
                                    if let Ok(session) = interview_session.lock() {
                                        if let Some(interview) = session.as_ref() {
                                            if let Ok(memory) = memory.lock() {
                                                if let Err(e) = rt.block_on(memory.save_turn(&transcription_for_engram, &response)) {
                                                    eprintln!("  ⚠️ Engram turn save failed: {}", e);
                                                } else {
                                                    println!("  🧠 Turn saved to Engram memory");
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("  ❌ LLM streaming failed: {}", e);
                                }
                            }
                        }
                    }
                }
                vad::VadEvent::Silence => {
                    if is_capturing {
                        speech_buffer.extend_from_slice(&chunk.samples);

                        // Partial transcription every ~2s of speech accumulation
                        // Emit partial transcription to HUD so user sees text live
                        let sample_rate = chunk.sample_rate;
                        let partial_interval = sample_rate as usize; // 1s at current rate
                        if speech_buffer.len() >= sample_rate as usize * 2
                            && speech_buffer.len() - last_partial_pos >= partial_interval
                        {
                            last_partial_pos = speech_buffer.len();

                            let partial_buf = speech_buffer.clone();
                            let cfg = config.clone();
                            let ah = app_handle.clone();
                            let sr = sample_rate;
                            let _ = std::thread::Builder::new()
                                .name("stt-partial".into())
                                .spawn(move || {
                                    if let Ok(wav) = stt::pcm_to_wav(&partial_buf, sr) {
                                        if let Ok(text) = stt::transcribe_local_sync(&cfg, &wav) {
                                            let trimmed = text.trim().to_string();
                                            if !trimmed.is_empty() {
                                                diag!("[DIAG] Partial STT OK: {}", &trimmed[..80.min(trimmed.len())]);
                                                emit_to_window(&ah, "partial-transcription",
                                                    TranscriptionPayload { text: trimmed });
                                            }
                                        }
                                    }
                                });
                        }
                    }
                }
            }
        }
    });
    Ok(())
}

/// Emit an event to the main window, with fallback to global emit.
fn emit_to_window<T: serde::Serialize + Clone>(app: &AppHandle, event: &str, payload: T) {
    use tauri::Emitter;
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit(event, payload);
    } else {
        println!("  ❌ Window 'main' not found for {}!", event);
        let _ = app.emit(event, payload);
    }
}