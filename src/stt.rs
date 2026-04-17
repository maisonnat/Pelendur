use crate::config::{Config, SttProvider};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::io::Cursor;
#[cfg(not(feature = "parakeet"))]
use std::process::Stdio;
#[cfg(not(feature = "parakeet"))]
use std::time::Instant;
#[cfg(not(feature = "parakeet"))]
use tokio::process::Command;
use tracing::debug;

#[cfg(feature = "parakeet")]
use crate::parakeet::ParakeetModel;

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
        let mut writer = hound::WavWriter::new(&mut cursor, spec)
            .context("Failed to create WAV writer")?;
        for &sample in samples {
            let sample_i16 = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer.write_sample(sample_i16)
                .context("Failed to write WAV sample")?;
        }
        writer.finalize()
            .context("Failed to finalize WAV")?;
    }

    Ok(cursor.into_inner())
}

/// Transcribe audio — routes to Groq API, z.ai API, or local whisper.cpp
pub async fn transcribe(config: &Config, audio_wav: &[u8]) -> Result<String> {
    match config.stt_provider {
        SttProvider::Groq => transcribe_groq(config, audio_wav).await,
        SttProvider::Zai => transcribe_zai(config, audio_wav).await,
        SttProvider::Local => transcribe_local(config, audio_wav).await,
    }
}

// ============================================================
// Local whisper.cpp via CLI
// ============================================================

/// Transcribe using local whisper.cpp CLI
#[cfg(not(feature = "parakeet"))]
async fn transcribe_local(config: &Config, audio_wav: &[u8]) -> Result<String> {
    let start = Instant::now();

    // Find whisper-cli binary
    let whisper_bin = find_whisper_binary()?;

    // Write audio to temp file
    let temp_dir = std::env::temp_dir();
    let temp_wav_name = format!("pelendur_audio_{}.wav", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0));
    let temp_wav = temp_dir.join(&temp_wav_name);
    tokio::fs::write(&temp_wav, audio_wav).await.context("Failed to write temp WAV file")?;

    // Validate model path exists
    let model_path = &config.whisper_model_path;
    if !std::path::Path::new(model_path).exists() {
        anyhow::bail!(
            "Whisper model not found at: {}\n\
             Download it with:\n\
               whisper.cpp: models/download-ggml-model.bat base.en\n\
             Or set WHISPER_MODEL_PATH in .env to the correct path.",
            model_path
        );
    }

    // Build whisper-cli command
    let threads = num_cpus().min(4); 

    let temp_wav_str = temp_wav.to_str()
        .context("Temp WAV path contains non-UTF8 characters")?;

    let output = Command::new(&whisper_bin)
        .args([
            "-m", model_path,
            "-f", temp_wav_str,
            "-l", &config.whisper_language,
            "--no-timestamps",
            "-t", &threads.to_string(),
            "--output-txt",
            "--output-file", temp_wav_str,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!(
            "Failed to spawn whisper-cli at: {}\n\
             Make sure whisper.cpp is compiled and whisper-cli is in your PATH.",
            whisper_bin.display()
        ))?
        .wait_with_output()
        .await?;

    // Read the text output file (whisper-cli adds .txt suffix to the --output-file path)
    let txt_file = temp_dir.join(format!("{}.txt", temp_wav_name));
    let text = if txt_file.exists() {
        let content = tokio::fs::read_to_string(&txt_file)
            .await
            .context("Failed to read whisper output")?;
        // Clean up
        let _ = tokio::fs::remove_file(&txt_file).await;
        content
    } else {
        // Fallback: parse stdout
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            debug!("whisper-cli stderr: {}", stderr);
        }

        stdout
            .lines()
            .filter_map(|line| {
                if let Some(pos) = line.find("] ") {
                    Some(line[pos + 2..].trim().to_string())
                } else {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() && !trimmed.starts_with('[') {
                        Some(trimmed.to_string())
                    } else {
                        None
                    }
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    };

    // Clean up temp WAV
    let _ = tokio::fs::remove_file(&temp_wav).await;

    let elapsed = start.elapsed();
    let text = text.trim().to_string();

    if text.is_empty() {
        debug!("whisper.cpp returned empty ({}ms)", elapsed.as_millis());
    } else {
        debug!("whisper.cpp: {} ({}ms)", text, elapsed.as_millis());
    }

    Ok(text)
}

/// Find whisper-cli binary in common locations
#[cfg(not(feature = "parakeet"))]
fn find_whisper_binary() -> Result<std::path::PathBuf> {
    // Check env var first
    if let Ok(path) = std::env::var("WHISPER_BIN") {
        let p = std::path::PathBuf::from(&path);
        if p.exists() {
            return Ok(p);
        }
    }

    // Common locations
    let candidates: Vec<String> = if cfg!(target_os = "windows") {
        vec![
            "whisper-cli.exe".to_string(),
            "whisper-cli.exe".to_string(),
            // Relative to common install locations
            format!("{}\\whisper.cpp\\build\\bin\\Release\\whisper-cli.exe",
                    std::env::var("USERPROFILE").unwrap_or_default()),
            format!("{}\\whisper.cpp\\build\\bin\\whisper-cli.exe",
                    std::env::var("USERPROFILE").unwrap_or_default()),
        ]
    } else {
        vec![
            "whisper-cli".to_string(),
            "/usr/local/bin/whisper-cli".to_string(),
            format!("{}/whisper.cpp/build/bin/whisper-cli",
                    std::env::var("HOME").unwrap_or_default()),
        ]
    };

    for candidate in &candidates {
        let path = std::path::PathBuf::from(candidate);
        if path.exists() {
            return Ok(path);
        }
    }

    // Try `which whisper-cli` / `where whisper-cli`
    let which_cmd = if cfg!(target_os = "windows") { "where" } else { "which" };
    if let Ok(output) = std::process::Command::new(which_cmd).arg("whisper-cli").output() {
        let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path_str.is_empty() {
            let path = std::path::PathBuf::from(&path_str);
            if path.exists() {
                return Ok(path);
            }
        }
    }

    anyhow::bail!(
        "whisper-cli not found!\n\
         \n\
         Install whisper.cpp:\n\
         1. git clone https://github.com/ggml-org/whisper.cpp.git\n\
         2. cd whisper.cpp\n\
         3. cmake -B build -DGGML_CUDA=1\n\
         4. cmake --build build -j --config Release\n\
         5. Download model: models/download-ggml-model.bat base.en\n\
         \n\
         Or set WHISPER_BIN and WHISPER_MODEL_PATH in .env"
    )
}

/// Get number of CPUs (simple)
#[cfg(not(feature = "parakeet"))]
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
async fn transcribe_local(_config: &Config, _audio_wav: &[u8]) -> Result<String> {
    anyhow::bail!("Use inference thread for Parakeet STT")
}

/// Transcribe using Parakeet ONNX model (called from dedicated inference thread).
/// Receives raw f32 samples directly — no WAV conversion needed.
#[cfg(feature = "parakeet")]
pub fn transcribe_parakeet_sync(model: &mut ParakeetModel, samples: Vec<f32>) -> Result<String> {
    let start = std::time::Instant::now();
    let result = model.transcribe_samples(samples)
        .map_err(|e| anyhow::anyhow!("Parakeet inference failed: {}", e))?;
    let elapsed = start.elapsed();
    if result.text.trim().is_empty() {
        tracing::debug!("Parakeet returned empty ({}ms)", elapsed.as_millis());
    } else {
        tracing::debug!("Parakeet: {} ({}ms)", result.text.trim(), elapsed.as_millis());
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
    let url = format!("{}/audio/transcriptions", config.openai_base_url.trim_end_matches('/'));

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
