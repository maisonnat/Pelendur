use crate::state::{AppState, AudioDevice, AudioLevelPayload, StreamWrapper, TranscriptionPayload, SuggestionPayload};
use ghostai_pilot::{audio, config, knowledge, llm, loopback, mixer, stt, vad};
use ghostai_pilot::llm::ChatMessage;
use tauri::{AppHandle, Manager, State, Emitter};
use cpal::traits::{DeviceTrait, HostTrait};

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

        let mut speech_buffer: Vec<f32> = Vec::with_capacity(16000 * 10);
        let mut is_capturing = false;

        while let Ok(chunk) = audio_rx.recv() {
            eprintln!("[DIAG] Chunk: {}samp {}Hz rms={:.4}",chunk.samples.len(),chunk.sample_rate,(chunk.samples.iter().map(|s|s*s).sum::<f32>()/chunk.samples.len() as f32).sqrt());
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

                            if let Ok(response) = rt.block_on(llm::generate_response(&config, &conversation)) {
                                println!("  🤖 IA: {}", response);
                                emit_to_window(&app_handle, "suggestion-update", SuggestionPayload { text: response.clone() });
                                conversation.push(ChatMessage { role: "assistant".to_string(), content: response.clone() });

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