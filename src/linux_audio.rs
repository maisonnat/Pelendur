//! Linux system audio capture via PulseAudio/PipeWire monitor source.
//!
//! Uses `parec` (PulseAudio) or `pw-cat` (PipeWire) subprocess to capture
//! from `@DEFAULT_MONITOR@`, which captures whatever audio is playing on
//! the default output device.

use crate::audio::AudioChunk;
use anyhow::{anyhow, Context, Result};
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Start capturing system audio via PulseAudio/PipeWire monitor.
///
/// Returns a channel that yields `AudioChunk` frames (16kHz mono f32).
/// Spawns a background thread that reads raw PCM from a `parec`/`pw-cat`
/// subprocess and converts s16le samples to f32.
pub fn start_system_audio_capture() -> Result<mpsc::Receiver<AudioChunk>> {
    let (tx, rx) = mpsc::channel();

    std::thread::Builder::new()
        .name("linux-audio-capture".to_string())
        .spawn(move || {
            if let Err(e) = capture_thread(tx) {
                error!("Linux audio capture error: {}", e);
            }
        })
        .context("Failed to spawn audio capture thread")?;

    Ok(rx)
}

fn capture_thread(tx: mpsc::Sender<AudioChunk>) -> Result<()> {
    let binary = which_parec()?;

    let sample_rate: u32 = 16000;
    let chunk_duration_secs: f32 = 1.0;
    let samples_per_chunk = (sample_rate as f32 * chunk_duration_secs) as usize;

    let args = build_args(&binary);

    info!(
        "Starting Linux audio capture via {} {} at {}Hz mono",
        binary,
        args.join(" "),
        sample_rate
    );

    let mut child = Command::new(&binary)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context("Failed to spawn audio capture subprocess")?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("Failed to get subprocess stdout"))?;

    let mut reader = std::io::BufReader::new(stdout);
    let bytes_per_sample: usize = 2; // s16le = 2 bytes per sample
    let mut buf = vec![0u8; samples_per_chunk * bytes_per_sample];

    loop {
        match reader.read_exact(&mut buf) {
            Ok(()) => {
                let samples: Vec<f32> = buf
                    .chunks_exact(bytes_per_sample)
                    .map(|chunk| {
                        let sample_i16 = i16::from_le_bytes([chunk[0], chunk[1]]);
                        sample_i16 as f32 / i16::MAX as f32
                    })
                    .collect();

                let chunk = AudioChunk {
                    samples,
                    sample_rate,
                };

                if tx.send(chunk).is_err() {
                    debug!("Audio channel closed, stopping capture");
                    break;
                }
            }
            Err(e) => {
                if let Ok(status) = child.try_wait() {
                    if status.is_some() {
                        warn!("Audio capture process exited: {:?}", status);
                        break;
                    }
                }
                return Err(anyhow!("Error reading from audio capture: {}", e));
            }
        }
    }

    let _ = child.kill();
    info!("Linux audio capture thread ended");
    Ok(())
}

/// Build CLI arguments depending on which binary we use.
fn build_args(binary: &str) -> Vec<&'static str> {
    if binary.ends_with("pw-cat") {
        // PipeWire native client: --format=s16, -r for record mode
        vec![
            "--format=s16",
            "--rate=16000",
            "--channels=1",
            "--target=@DEFAULT_MONITOR@",
            "-r",
        ]
    } else {
        // PulseAudio parec (also works via PipeWire's PulseAudio compat)
        vec![
            "--format=s16le",
            "--rate=16000",
            "--channels=1",
            "--device=@DEFAULT_MONITOR@",
        ]
    }
}

/// Locate a suitable audio capture binary.
///
/// Tries `parec` first (PulseAudio / PipeWire PulseAudio compat layer),
/// then falls back to `pw-cat` (native PipeWire).
fn which_parec() -> Result<String> {
    if let Ok(output) = Command::new("which").arg("parec").output() {
        if output.status.success() {
            return Ok("parec".to_string());
        }
    }
    if let Ok(output) = Command::new("which").arg("pw-cat").output() {
        if output.status.success() {
            return Ok("pw-cat".to_string());
        }
    }
    Err(anyhow!(
        "Neither parec nor pw-cat found. Install PulseAudio or PipeWire:\n\
         Ubuntu/Debian: sudo apt install pulseaudio-utils\n\
         Fedora: sudo dnf install pulseaudio-utils"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_args_parec() {
        let args = build_args("parec");
        assert!(args.contains(&"--format=s16le"));
        assert!(args.contains(&"--rate=16000"));
        assert!(args.contains(&"--channels=1"));
        assert!(args.contains(&"--device=@DEFAULT_MONITOR@"));
        assert!(!args.contains(&"-r"));
    }

    #[test]
    fn test_build_args_pwcat() {
        let args = build_args("pw-cat");
        assert!(args.contains(&"--format=s16"));
        assert!(args.contains(&"--rate=16000"));
        assert!(args.contains(&"--channels=1"));
        assert!(args.contains(&"--target=@DEFAULT_MONITOR@"));
        assert!(args.contains(&"-r"));
        assert!(!args.contains(&"--device"));
    }

    #[test]
    fn test_build_args_parec_binary_path() {
        let args = build_args("/usr/bin/parec");
        assert!(args.contains(&"--format=s16le"));
        assert!(!args.contains(&"-r"));
    }

    #[test]
    fn test_s16le_to_f32_conversion() {
        let max_f32 = 32767_i16 as f32 / i16::MAX as f32;
        assert!((max_f32 - 1.0).abs() < 0.001);

        let min_f32 = (-32768_i16) as f32 / i16::MAX as f32;
        assert!(min_f32 <= -1.0);

        let zero_f32 = 0_i16 as f32 / i16::MAX as f32;
        assert!(zero_f32.abs() < 0.0001);
    }

    #[test]
    fn test_which_parec_graceful_error() {
        let result = which_parec();
        if let Err(e) = result {
            let msg = e.to_string();
            assert!(msg.contains("parec") || msg.contains("pw-cat"));
            assert!(msg.contains("apt install") || msg.contains("dnf install"));
        }
    }
}
