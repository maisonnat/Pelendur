use crate::state::{AppState, AudioDevice, StreamWrapper, TranscriptionPayload, SuggestionPayload};
use ghostai_pilot::{audio, config, knowledge, llm, loopback, stt, vad};
use ghostai_pilot::llm::ChatMessage;
use tauri::{AppHandle, Manager, State, Emitter};
use cpal::traits::{DeviceTrait, HostTrait};

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

    // Determine capture source: "system" = WASAPI loopback, "mic" = cpal microphone
    let capture_mode = mode.unwrap_or_else(|| {
        // Default to system loopback on Windows, mic on Linux
        if cfg!(target_os = "windows") { "system".to_string() } else { "mic".to_string() }
    });

    let audio_rx = if capture_mode == "system" {
        // WASAPI loopback — captures all audio playing through the output device
        println!("  🔊 Starting WASAPI loopback capture (system audio)...");
        loopback::start_system_loopback_capture().map_err(|e| format!("Loopback failed: {}", e))?
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

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut vad_detector = vad::VadDetector::default_config();

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

                            conversation.push(ChatMessage { role: "user".to_string(), content: transcription });

                            if let Ok(response) = rt.block_on(llm::generate_response(&config, &conversation)) {
                                println!("  🤖 IA: {}", response);
                                emit_to_window(&app_handle, "suggestion-update", SuggestionPayload { text: response.clone() });
                                conversation.push(ChatMessage { role: "assistant".to_string(), content: response.clone() });

                                // Store the turn to Engram if in interview mode
                                if let Ok(session) = interview_session.lock() {
                                    if let Some(interview) = session.as_ref() {
                                        let company = interview.company.clone();
                                        // TODO: Restore Engram turn save with async context
                                        // Currently skipped because MutexGuard can't cross .await
                                        eprintln!("  ⚠️ Engram save deferred (async context)");
                                        // Engram turn save handled by C4 Conversation Memory manager
                                        // Update turn count in interview session
                                        if let Ok(mut session2) = interview_session.lock() {
                                            if let Some(ref mut s) = *session2 {
                                                s.turn_count += 1;
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