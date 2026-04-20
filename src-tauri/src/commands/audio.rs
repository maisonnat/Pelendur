use crate::state::{AppState, AudioDevice, StreamWrapper, TranscriptionPayload, SuggestionPayload};
use ghostai_pilot::{audio, audio_config, config, knowledge, llm, loopback, stt, vad};
use ghostai_pilot::llm::ChatMessage;
use tauri::{AppHandle, Manager, State, WebviewWindow, Emitter};
use cpal::traits::{DeviceTrait, HostTrait};

#[tauri::command]
pub fn get_audio_processes() -> Result<Vec<loopback::real::AudioProcess>, String> {
    Ok(loopback::real::list_audio_processes())
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
    pid: Option<u32>,
    device_index: Option<usize>,
) -> Result<(), String> {
    let config = state.config.clone();
    let km_lock = state.knowledge_manager.clone();
    let conversation_lock = state.conversation.clone();
    let streams_lock = state.active_streams.clone();

    {
        let mut streams = streams_lock.lock().map_err(|e| e.to_string())?;
        streams.clear();
    }

    let (audio_rx, stream) = if let Some(_pid) = pid {
        // pid parameter kept for API compatibility, but wasapi_loopback feature not active
        let strategy = audio_config::detect_strategy().map_err(|e| e.to_string())?;
        let rx = strategy.start_system_capture().map_err(|e| e.to_string())?;
        (rx, None)
    } else {
        let host = cpal::default_host();
        let devices: Vec<_> = host.input_devices().map_err(|e| e.to_string())?.collect();
        let device = if let Some(idx) = device_index {
            devices.get(idx).ok_or_else(|| "Invalid index".to_string())?.clone()
        } else {
            audio::find_microphone_device().map_err(|e| e.to_string())?
        };
        println!("  ⚙ Captura: {:?}", device.name().unwrap_or_default());
        let (rx, stream) = audio::start_capture(device).map_err(|e| e.to_string())?;
        (rx, Some(stream))
    };

    if let Some(s) = stream {
        let mut streams = streams_lock.lock().map_err(|e| e.to_string())?;
        streams.push(StreamWrapper(s));
    }

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
                                conversation.push(ChatMessage { role: "assistant".to_string(), content: response });
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
