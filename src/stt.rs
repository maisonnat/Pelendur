use crate::config::{Config, SttProvider};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::io::Cursor;
use std::sync::{Mutex, OnceLock};
use std::sync::mpsc;
use std::time::Instant;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Append a diagnostic line to pelendur-pipe.log (works inside spawn_blocking)
#[cfg(not(feature = "parakeet"))]
fn diag_log(msg: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("pelendur-pipe.log") {
        let _ = writeln!(f, "[STT] {}", msg);
    }
}

#[cfg(feature = "parakeet")]
use crate::parakeet::ParakeetModel;

#[cfg(feature = "parakeet")]
static PARAKEET_MODEL: OnceLock<Mutex<ParakeetModel>> = OnceLock::new();

/// Inference request types for the dedicated inference thread.
#[cfg(feature = "parakeet")]
pub enum InferenceRequest {
    Partial { samples: Vec<f32> },
    Final { samples: Vec<f32>, response_tx: tokio::sync::oneshot::Sender<Result<String>> },
}

/// Spawn a dedicated Parakeet inference thread (streaming mode).
/// Returns (sender, partial_receiver).
#[cfg(feature = "parakeet")]
pub fn spawn_parakeet_inference(
    model: ParakeetModel,
) -> (mpsc::Sender<InferenceRequest>, broadcast::Receiver<String>) {
    let (tx, rx) = mpsc::channel::<InferenceRequest>();
    let (partial_tx, partial_rx) = broadcast::channel::<String>(16);
    let partial_tx_clone = partial_tx.clone();

    std::thread::Builder::new()
        .name("parakeet-inference".into())
        .spawn(move || {
            let mut model = model;
            while let Ok(req) = rx.recv() {
                match req {
                    InferenceRequest::Partial { samples } => {
                        if let Ok(text) = transcribe_parakeet_sync(&mut model, samples) {
                            if !text.trim().is_empty() {
                                let _ = partial_tx_clone.send(text);
                            }
                        }
                    }
                    InferenceRequest::Final { samples, response_tx } => {
                        let result = transcribe_parakeet_sync(&mut model, samples);
                        let _ = response_tx.send(result);
                    }
                }
            }
        })
        .expect("Failed to spawn Parakeet inference thread");

    (tx, partial_rx)
}

/// Initialize the global Parakeet model (call once at startup).
#[cfg(feature = "parakeet")]
pub fn init_parakeet_model(model_dir: &std::path::Path, use_encoder: bool) -> Result<()> {
    let model = ParakeetModel::new(model_dir, use_encoder)
        .map_err(|e| anyhow::anyhow!("Failed to init Parakeet model: {}", e))?;
    PARAKEET_MODEL
        .set(Mutex::new(model))
        .map_err(|_| anyhow::anyhow!("Parakeet model already initialized"))?;
    tracing::info!("Global Parakeet model initialized");
    Ok(())
}

/// Get a reference to the global Parakeet model.
#[cfg(feature = "parakeet")]
pub fn get_parakeet_model() -> Result<&'static Mutex<ParakeetModel>> {
    PARAKEET_MODEL
        .get()
        .ok_or_else(|| anyhow::anyhow!("Parakeet model not initialized"))
}

/// Encode PCM f32 samples into WAV bytes (16kHz mono, 16-bit)
pub fn pcm_to_wav(samples: &[f32], sample_rate: u32) -> Result<Vec<u8>> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer =
            hound::WavWriter::new(&mut cursor, spec).context("Failed to create WAV writer")?;
        for &sample in samples {
            let sample_i16 = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer
                .write_sample(sample_i16)
                .context("Failed to write WAV sample")?;
        }
        writer.finalize().context("Failed to finalize WAV")?;
    }

    Ok(cursor.into_inner())
}

/// Apply silence suppression to audio samples.
/// Removes low-energy segments (background noise) between words
/// while preserving word boundaries with a small hangover.
/// Improves STT accuracy in environments with keyboard noise,
/// traffic, or air conditioning.
///
/// Parameters:
/// - `samples`: PCM f32 samples in [-1.0, 1.0] range
/// - `sample_rate`: sample rate in Hz (e.g. 16000, 48000)
/// - `threshold`: RMS energy threshold (default 0.01 ≈ -40dB);
///   frames below this are considered silence
pub fn apply_silence_suppression(samples: &[f32], sample_rate: u32, threshold: f32) -> Vec<f32> {
    const FRAME_MS: u32 = 30;          // 30ms frames for energy computation
    const HANGOVER_FRAMES: usize = 4;  // ~120ms padding at word edges

    let frame_size = (sample_rate * FRAME_MS / 1000) as usize;
    if frame_size == 0 || samples.len() < frame_size {
        return samples.to_vec();
    }

    let num_frames = samples.len() / frame_size;
    let mut is_active = vec![false; num_frames];

    // 1. Compute RMS energy per frame
    for i in 0..num_frames {
        let start = i * frame_size;
        let end = (start + frame_size).min(samples.len());
        let frame = &samples[start..end];
        let rms = (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt();
        is_active[i] = rms > threshold;
    }

    // 2. Forward pass: extend active regions (hangover at word endings)
    let mut extended = is_active.clone();
    for i in 0..num_frames {
        if is_active[i] {
            for j in 1..=HANGOVER_FRAMES {
                if i + j < num_frames {
                    extended[i + j] = true;
                }
            }
        }
    }

    // 3. Backward pass: fill short gaps between words (< ~240ms of silence)
    let mut final_active = extended.clone();
    for i in 0..num_frames {
        if !extended[i] {
            // Check distance to nearest active frame before and after
            let distance_before = (1..=HANGOVER_FRAMES * 2)
                .find(|j| i >= *j && extended[i - j])
                .unwrap_or(usize::MAX);
            let distance_after = (1..=HANGOVER_FRAMES * 2)
                .find(|j| i + j < num_frames && extended[i + j])
                .unwrap_or(usize::MAX);
            // Fill if it's a short breath pause between words
            if distance_before != usize::MAX
                && distance_after != usize::MAX
                && distance_before + distance_after <= HANGOVER_FRAMES * 2
            {
                final_active[i] = true;
            }
        }
    }

    // 4. Extract only the active frames
    let mut result = Vec::with_capacity(samples.len());
    for i in 0..num_frames {
        if final_active[i] {
            let start = i * frame_size;
            let end = (start + frame_size).min(samples.len());
            result.extend_from_slice(&samples[start..end]);
        }
    }

    // Log compression ratio for diagnostics
    let compression = if samples.len() > 0 {
        ((samples.len() - result.len()) as f64 / samples.len() as f64 * 100.0) as u32
    } else {
        0
    };
    if compression > 0 {
        debug!("silence suppression: {}% compression ({} -> {})",
            compression, samples.len(), result.len());
    }

    result
}

// ============================================================
// whisper-rs — native Rust bindings (no subprocess)
// ============================================================

static WHISPER_RS_CTX: OnceLock<Mutex<WhisperContext>> = OnceLock::new();

/// Initialize whisper-rs model globally (call once at startup).
/// Loads the GGML model into memory and keeps it hot for all
/// subsequent transcriptions. Eliminates the ~50ms subprocess
/// startup overhead per STT call (subprocess whisper-cli.exe is gone).
///
/// When compiled with `--features cuda`, enables GPU acceleration
/// (NVIDIA CUDA) on device 0 with flash attention.
pub fn init_whisper_rs(model_path: &str) -> Result<()> {
    let path = std::path::Path::new(model_path);
    if !path.exists() {
        anyhow::bail!(
            "Whisper model not found at: {}\n\
             Download it with:\n\
               whisper.cpp: models/download-ggml-model.bat base.en\n\
             Or set WHISPER_MODEL_PATH in .env to the correct path.",
            model_path
        );
    }

    #[cfg(feature = "cuda")]
    let ctx = {
        let mut params = WhisperContextParameters::new();
        params.use_gpu(true);
        params.gpu_device(0);
        params.flash_attn(true);
        info!("whisper-rs: loading model with CUDA GPU acceleration (device 0)");
        WhisperContext::new_with_params(model_path, params)
    };

    #[cfg(not(feature = "cuda"))]
    let ctx = {
        info!("whisper-rs: loading model (CPU, no GPU feature)");
        WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
    };

    let ctx = ctx
        .map_err(|e| anyhow::anyhow!(
            "Failed to load whisper-rs model from {}: {}", model_path, e))?;
    WHISPER_RS_CTX
        .set(Mutex::new(ctx))
        .map_err(|_| anyhow::anyhow!("whisper-rs already initialized"))?;
    info!("whisper-rs model loaded: {} (exists={})", model_path, path.exists());
    Ok(())
}

/// Get a reference to the global whisper-rs model context.
fn get_whisper_rs_ctx() -> Result<&'static Mutex<WhisperContext>> {
    WHISPER_RS_CTX
        .get()
        .ok_or_else(|| anyhow::anyhow!(
            "whisper-rs not initialized. Call init_whisper_rs() first."
        ))
}

/// Transcribe audio using whisper-rs native bindings — fully synchronous.
/// No subprocess overhead. The model is loaded once at startup and reused
/// for every call. Call from a std::thread directly.
#[cfg(not(feature = "parakeet"))]
pub fn transcribe_local_sync(config: &Config, audio_wav: &[u8]) -> Result<String> {
    let start = Instant::now();

    // Decode WAV bytes to f32 PCM samples
    let mut reader = hound::WavReader::new(Cursor::new(audio_wav))
        .context("Failed to read WAV for whisper-rs")?;
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .filter_map(|s| s.ok())
        .map(|s| s as f32 / i16::MAX as f32)
        .collect();

    // Get the global model context (loaded once at startup)
    let ctx = get_whisper_rs_ctx()?;
    let mut ctx = ctx.lock()
        .map_err(|e| anyhow::anyhow!("whisper-rs mutex poisoned: {}", e))?;

    // Configure inference parameters — matches what whisper-cli did
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_n_threads(num_cpus().min(4).try_into().unwrap());
    params.set_language(Some(&config.whisper_language));
    params.set_no_timestamps(true);
    params.set_suppress_non_speech_tokens(true);

    // Create a computation state from the model context
    let mut state = ctx.create_state()
        .map_err(|e| anyhow::anyhow!("whisper-rs create_state failed: {}", e))?;
    // Drop the mutex lock — state is fully independent after creation
    drop(ctx);

    // Run inference (this is where the real work happens)
    state.full(params, &samples)
        .map_err(|e| anyhow::anyhow!("whisper-rs inference failed: {}", e))?;

    // Collect all transcribed segments into one string
    let num_segments = state.full_n_segments()
        .map_err(|e| anyhow::anyhow!("whisper-rs get segments failed: {}", e))?;
    let mut text = String::with_capacity(1024);
    for i in 0..num_segments {
        let segment = state.full_get_segment_text(i)
            .map_err(|e| anyhow::anyhow!("whisper-rs segment {} failed: {}", i, e))?;
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(&segment);
    }

    let elapsed = start.elapsed();
    let text = text.trim().to_string();

    if text.is_empty() {
        debug!("whisper-rs returned empty ({}ms)", elapsed.as_millis());
    } else {
        debug!("whisper-rs: \"{}\" ({}ms)", text, elapsed.as_millis());
    }

    Ok(text)
}

/// Transcribe audio — routes to Groq API, z.ai API, or local whisper-rs / Parakeet
pub async fn transcribe(config: &Config, audio_wav: &[u8]) -> Result<String> {
    match config.stt_provider {
        SttProvider::Groq => transcribe_groq(config, audio_wav).await,
        SttProvider::Zai => transcribe_zai(config, audio_wav).await,
        SttProvider::Local => {
            #[cfg(feature = "parakeet")]
            {
                // Parakeet ONNX path — async, uses global model
                transcribe_local(config, audio_wav).await
            }
            #[cfg(not(feature = "parakeet"))]
            {
                // whisper-rs path — sync via spawn_blocking
                let config = config.clone();
                let wav = audio_wav.to_vec();
                tokio::task::spawn_blocking(move || transcribe_local_sync(&config, &wav))
                    .await
                    .context("spawn_blocking panicked")?
            }
        }
    }
}

/// Get number of CPUs (simple)
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

// ============================================================
// Parakeet ONNX STT (local)
// ============================================================

/// Stub — Parakeet routes through the dedicated inference thread, not here.
#[cfg(feature = "parakeet")]
async fn transcribe_local(config: &Config, audio_wav: &[u8]) -> Result<String> {
    use hound::WavReader;
    use std::io::Cursor;
    // Decode WAV bytes back to f32 samples
    let mut reader = WavReader::new(Cursor::new(audio_wav))
        .context("Failed to read WAV for Parakeet")?;
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .filter_map(|s| s.ok())
        .map(|s| s as f32 / i16::MAX as f32)
        .collect();
    let mut model = get_parakeet_model()
        .map_err(|e| anyhow::anyhow!("Parakeet model not available: {}", e))?
        .lock()
        .map_err(|e| anyhow::anyhow!("Parakeet model lock failed: {}", e))?;
    transcribe_parakeet_sync(&mut model, samples)
}

/// Transcribe using Parakeet ONNX model (called from dedicated inference thread).
/// Receives raw f32 samples directly — no WAV conversion needed.
/// Automatically resamples to 16kHz if needed (Parakeet expects 16kHz input).
#[cfg(feature = "parakeet")]
pub fn transcribe_parakeet_sync(model: &mut ParakeetModel, samples: Vec<f32>) -> Result<String> {
    let start = std::time::Instant::now();
    // Resample to 16kHz: if >16000 samples, it's likely 48kHz, decimate 3:1
    let samples = if samples.len() > 16000 * 2 {
        samples.into_iter().step_by(3).collect::<Vec<f32>>()
    } else {
        samples
    };
    tracing::debug!("Parakeet infer: {} input → {} after 16kHz decimation", 
        std::time::Instant::now().duration_since(start).as_millis(), samples.len());
    let result = model
        .transcribe_samples(samples)
        .map_err(|e| anyhow::anyhow!("Parakeet inference failed: {}", e))?;
    let elapsed = start.elapsed();
    if result.text.trim().is_empty() {
        tracing::debug!("Parakeet returned empty ({}ms)", elapsed.as_millis());
    } else {
        tracing::debug!(
            "Parakeet: {} ({}ms)",
            result.text.trim(),
            elapsed.as_millis()
        );
    }
    Ok(result.text.trim().to_string())
}

// ============================================================
// Groq Whisper API (cloud)
// ============================================================

async fn transcribe_groq(config: &Config, audio_wav: &[u8]) -> Result<String> {
    let url = "https://api.groq.com/openai/v1/audio/transcriptions";

    let part = reqwest::multipart::Part::bytes(audio_wav.to_vec())
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .context("Failed to create multipart part")?;

    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model", config.groq_stt_model.clone())
        .text("language", config.whisper_language.clone())
        .text("response_format", "json");

    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", config.groq_api_key))
        .multipart(form)
        .send()
        .await
        .context("Failed to send STT request to Groq")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Groq STT API error {}: {}", status, body);
    }

    #[derive(Deserialize)]
    struct TranscriptionResponse {
        text: String,
    }

    let result: TranscriptionResponse = response
        .json()
        .await
        .context("Failed to parse STT response")?;

    let text = result.text.trim().to_string();
    debug!("Groq STT: {}", text);
    Ok(text)
}

// ============================================================
// z.ai ASR API (cloud)
// ============================================================

async fn transcribe_zai(config: &Config, audio_wav: &[u8]) -> Result<String> {
    let url = format!(
        "{}/audio/transcriptions",
        config.openai_base_url.trim_end_matches('/')
    );

    let part = reqwest::multipart::Part::bytes(audio_wav.to_vec())
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .context("Failed to create multipart part")?;

    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model", "glm-asr-2512".to_string())
        .text("language", config.whisper_language.clone())
        .text("response_format", "json");

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.openai_api_key))
        .multipart(form)
        .send()
        .await
        .context("Failed to send STT request to z.ai")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("z.ai STT API error {}: {}", status, body);
    }

    #[derive(Deserialize)]
    struct TranscriptionResponse {
        text: String,
    }

    let result: TranscriptionResponse = response
        .json()
        .await
        .context("Failed to parse z.ai STT response")?;

    let text = result.text.trim().to_string();
    debug!("z.ai STT: {}", text);
    Ok(text)
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_silence_suppression_preserves_loud_audio() {
        let sample_rate: u32 = 16000;
        let num_samples = (sample_rate / 5) as usize;
        let samples: Vec<f32> = (0..num_samples)
            .map(|i| (i as f32 * 440.0 * 2.0 * std::f32::consts::PI / sample_rate as f32).sin())
            .collect();
        let result = apply_silence_suppression(&samples, sample_rate, 0.01);
        assert!(!result.is_empty(), "should not remove loud audio");
        assert!(result.len() > samples.len() / 2, "should keep most of loud audio");
    }

    #[test]
    fn test_silence_suppression_removes_silence() {
        let sample_rate: u32 = 16000;
        let samples = vec![0.0f32; sample_rate as usize];
        let result = apply_silence_suppression(&samples, sample_rate, 0.01);
        assert!(result.is_empty() || result.len() < 10,
            "should remove silence, got {} samples", result.len());
    }

    #[test]
    fn test_silence_suppression_short_buffer() {
        let samples = vec![0.5f32; 10];
        let result = apply_silence_suppression(&samples, 16000, 0.01);
        assert_eq!(result.len(), 10, "short buffers should pass through");
    }

    #[test]
    fn test_silence_suppression_keeps_short_pauses() {
        let sample_rate: u32 = 16000;
        let word = vec![0.5f32; sample_rate as usize / 4];
        let pause = vec![0.0f32; sample_rate as usize / 10];
        let mut samples = Vec::new();
        samples.extend(&word);
        samples.extend(&pause);
        samples.extend(&word);
        let result = apply_silence_suppression(&samples, sample_rate, 0.01);
        assert!(!result.is_empty(), "should keep speech with short pauses");
        assert!(result.len() > word.len(), "should keep at least one word");
    }
}
