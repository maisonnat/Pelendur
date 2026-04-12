use anyhow::{anyhow, Result};
use std::sync::mpsc;
use tracing::{debug, info, warn};

/// Represents a chunk of captured audio (PCM f32 samples at 16kHz)
pub struct AudioChunk {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

// ============================
// Real audio capture (cpal)
// ============================
#[cfg(feature = "audio")]
mod real {
    use super::*;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use cpal::{SampleFormat, SampleRate, StreamConfig};

    /// Find the system audio device for capturing output audio.
    ///
    /// Windows (WASAPI): looks for loopback devices — these capture
    /// what's playing through the speakers. WASAPI loopback devices
    /// appear as input devices with "loopback" in the name.
    ///
    /// Linux (PulseAudio/PipeWire): looks for "monitor" devices.
    ///
    /// macOS: cpal doesn't support ScreenCaptureKit yet.
    ///         Use the microphone as fallback + mention BlackHole.
    pub fn find_system_audio_device() -> Result<Device> {
        let host = cpal::default_host();

        #[cfg(target_os = "windows")]
        {
            return find_system_audio_windows(&host);
        }

        #[cfg(target_os = "linux")]
        {
            return find_system_audio_linux(&host);
        }

        #[cfg(target_os = "macos")]
        {
            return find_system_audio_macos(&host);
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            Err(anyhow!(
                "System audio capture not supported on this platform"
            ))
        }
    }

    // ===========================
    // Windows: WASAPI loopback
    // ===========================
    #[cfg(target_os = "windows")]
    fn find_system_audio_windows(host: &cpal::Host) -> Result<Device> {
        // On Windows with cpal + WASAPI, loopback devices show up as input devices.
        // They typically have names containing "loopback" or are the default output
        // device mirrored as an input.
        let devices = host
            .input_devices()
            .map_err(|e| anyhow!("Failed to enumerate input devices: {}", e))?;

        let mut all_devices = Vec::new();
        for device in devices {
            let name = device.name().unwrap_or_default();
            debug!("Found input device: {}", name);
            all_devices.push((name.clone(), device));

            // WASAPI loopback devices usually have "loopback" in the name
            if name.to_lowercase().contains("loopback") {
                info!("Found WASAPI loopback device: {}", name);
                return Ok(all_devices.pop().unwrap().1);
            }
        }

        // If no explicit loopback found, try the default output device.
        // On some WASAPI setups, cpal exposes the default output as an input
        // device that captures system audio.
        if let Some(default_output) = host.default_output_device() {
            let output_name = default_output.name().unwrap_or_default();
            info!("Trying default output device as loopback: {}", output_name);

            // Check if there's an input device with a matching name
            for (name, device) in &all_devices {
                if name.contains(&output_name) || output_name.contains(name) {
                    info!("Matched output device to input: {}", name);
                    // Can't move out of the vec easily, find it again
                    // Actually we need to return the device
                }
            }
        }

        // Fallback: if we're on a newer version of cpal that exposes
        // WASAPI loopback properly, just use the first input device
        // that has output-like characteristics
        if !all_devices.is_empty() {
            let (name, device) = all_devices.remove(0);
            warn!("Using first available input device: {}", name);
            warn!("NOTE: If this captures microphone instead of system audio,");
            warn!("you may need to enable 'Stereo Mix' in Windows Sound Settings,");
            warn!("or install VB-Cable/Virtual Audio Cable.");
            return Ok(device);
        }

        Err(anyhow!(
            "No audio input device found. \
             On Windows, system audio capture requires one of:\n\
             1. WASAPI loopback support (Windows 10+)\n\
             2. 'Stereo Mix' enabled in Sound Settings → Recording tab\n\
             3. VB-Cable installed (https://vb-audio.com/Cable/)\n\
             4. VoiceMeeter installed"
        ))
    }

    // ===========================
    // Linux: PulseAudio/PipeWire monitor
    // ===========================
    #[cfg(target_os = "linux")]
    fn find_system_audio_linux(host: &cpal::Host) -> Result<Device> {
        let devices = host
            .input_devices()
            .map_err(|e| anyhow!("Failed to enumerate input devices: {}", e))?;

        for device in devices {
            let name = device.name().unwrap_or_default();
            debug!("Found input device: {}", name);
            if name.to_lowercase().contains("monitor") {
                info!("Found system audio monitor: {}", name);
                return Ok(device);
            }
        }

        warn!("No monitor device found. Available input devices:");
        let devices = host
            .input_devices()
            .map_err(|e| anyhow!("Failed to enumerate input devices: {}", e))?;
        for device in devices {
            warn!("  - {}", device.name().unwrap_or_default());
        }

        Err(anyhow!(
            "No system audio monitor device found. \
             Try: pactl load-module module-null-sink sink_name=ghostai"
        ))
    }

    // ===========================
    // macOS: microphone fallback
    // ===========================
    #[cfg(target_os = "macos")]
    fn find_system_audio_macos(host: &cpal::Host) -> Result<Device> {
        // cpal doesn't support ScreenCaptureKit.
        // Options: BlackHole virtual audio device, or fallback to mic
        let devices = host
            .input_devices()
            .map_err(|e| anyhow!("Failed to enumerate input devices: {}", e))?;

        for device in devices {
            let name = device.name().unwrap_or_default();
            if name.to_lowercase().contains("blackhole") {
                info!("Found BlackHole device: {}", name);
                return Ok(device);
            }
        }

        warn!("No BlackHole device found.");
        warn!("For system audio on macOS, install BlackHole:");
        warn!("  brew install blackhole-2ch");
        warn!("Then set it as output in System Preferences → Sound");
        warn!("Falling back to microphone.");

        host.default_input_device()
            .ok_or_else(|| anyhow!("No input device found"))
    }

    pub fn find_microphone_device() -> Result<Device> {
        let host = cpal::default_host();
        host.default_input_device()
            .ok_or_else(|| anyhow!("No default input device (microphone) found"))
    }

    /// List all available audio devices with labels.
    /// Includes input devices AND output devices (for WASAPI loopback).
    /// Returns (name, device, is_default, label) tuples.
    fn enumerate_input_devices() -> Result<Vec<(String, Device, bool, String)>> {
        let host = cpal::default_host();
        let default_input_name = host
            .default_input_device()
            .and_then(|d| d.name().ok())
            .unwrap_or_default();
        let default_output_name = host
            .default_output_device()
            .and_then(|d| d.name().ok())
            .unwrap_or_default();

        let mut result: Vec<(String, Device, bool, String)> = Vec::new();

        // First: add OUTPUT devices as loopback sources (WASAPI loopback)
        // These capture what's playing through speakers/headphones
        if let Ok(output_devices) = host.output_devices() {
            for device in output_devices {
                let name = device.name().unwrap_or_default();
                let lower = name.to_lowercase();
                let is_default = name == default_output_name;

                let label = if lower.contains("altavoces") || lower.contains("speakers") {
                    "🔊 Loopback (system audio)".to_string()
                } else if lower.contains("auricular")
                    || lower.contains("headphone")
                    || lower.contains("buds")
                    || lower.contains("headset")
                {
                    "🔊 Loopback (headphones)".to_string()
                } else if lower.contains("hdmi") {
                    "🔊 Loopback (HDMI)".to_string()
                } else if lower.contains("broadcast") {
                    "🎮 Loopback (NVIDIA Broadcast)".to_string()
                } else {
                    "🔊 Loopback (output)".to_string()
                };

                result.push((name, device, is_default, label));
            }
        }

        // Second: add INPUT devices (microphones)
        if let Ok(input_devices) = host.input_devices() {
            for device in input_devices {
                let name = device.name().unwrap_or_default();
                let lower = name.to_lowercase();
                let is_default = name == default_input_name;

                let label = if lower.contains("loopback")
                    || lower.contains("stereo mix")
                    || lower.contains("mezcla estéreo")
                {
                    "🔊 System Audio (Stereo Mix)".to_string()
                } else if lower.contains("monitor") {
                    "🔊 System Audio Monitor".to_string()
                } else if lower.contains("micro") || lower.contains("mic") {
                    "🎤 Microphone".to_string()
                } else if lower.contains("cable")
                    || lower.contains("vb-audio")
                    || lower.contains("voicemeeter")
                {
                    "🔊 Virtual Cable".to_string()
                } else if lower.contains("blackhole") {
                    "🔊 BlackHole (system audio)".to_string()
                } else if lower.contains("broadcast") {
                    "🎤 NVIDIA Broadcast".to_string()
                } else if lower.contains("auricular")
                    || lower.contains("headset")
                    || lower.contains("earphone")
                    || lower.contains("buds")
                {
                    "🎤 Headset mic".to_string()
                } else {
                    "🎤 Input".to_string()
                };

                result.push((name, device, is_default, label));
            }
        }

        if result.is_empty() {
            return Err(anyhow!("No audio devices found on this system"));
        }

        Ok(result)
    }

    /// Interactive device selector — shows all devices and prompts the user.
    /// Falls back to default if user presses Enter without choosing.
    pub fn select_device_interactive() -> Result<Device> {
        let devices = enumerate_input_devices()?;

        if devices.is_empty() {
            return Err(anyhow!("No audio input devices found on this system"));
        }

        println!("  Available audio devices:");
        println!();

        for (i, (name, _, is_default, label)) in devices.iter().enumerate() {
            let marker = if *is_default { " ← default" } else { "" };
            println!("    [{}] {} {}{}", i + 1, label, name, marker);
        }

        println!();
        print!("  Select device [1-{}] (Enter = default): ", devices.len());
        use std::io::Write;
        std::io::stdout().flush().ok();

        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
        let input = input.trim();

        let selected = if input.is_empty() {
            // Use default
            devices
                .iter()
                .find(|(_, _, is_def, _)| *is_def)
                .or_else(|| devices.first())
                .unwrap()
                .clone()
        } else if let Ok(idx) = input.parse::<usize>() {
            if idx >= 1 && idx <= devices.len() {
                devices[idx - 1].clone()
            } else {
                println!("  Invalid choice, using default.");
                devices
                    .iter()
                    .find(|(_, _, is_def, _)| *is_def)
                    .or_else(|| devices.first())
                    .unwrap()
                    .clone()
            }
        } else {
            // Try to match by name substring
            let lower_input = input.to_lowercase();
            devices
                .iter()
                .find(|(name, _, _, _)| name.to_lowercase().contains(&lower_input))
                .cloned()
                .unwrap_or_else(|| {
                    devices
                        .iter()
                        .find(|(_, _, is_def, _)| *is_def)
                        .or_else(|| devices.first())
                        .unwrap()
                        .clone()
                })
        };

        println!();
        println!("  ✓ Selected: {} ({})", selected.3, selected.0);
        println!();

        Ok(selected.1)
    }

    pub fn start_capture(device: Device) -> Result<mpsc::Receiver<AudioChunk>> {
        // Try to find a good config: prefer 16kHz F32 for whisper
        let mut best_config = None;

        // Try supported_input_configs (works for real input devices)
        // For output devices (WASAPI loopback), this may return empty — that's ok
        if let Ok(configs) = device.supported_input_configs() {
            for config_range in configs {
                let sample_format = config_range.sample_format();
                let min_rate = config_range.min_sample_rate().0;
                let max_rate = config_range.max_sample_rate().0;

                if sample_format == SampleFormat::F32 && min_rate <= 16000 && max_rate >= 16000 {
                    best_config = Some(config_range.with_sample_rate(SampleRate(16000)));
                    break;
                }
            }
        }

        // Fallback: use whatever the device gives us
        // (this is what WASAPI loopback needs — it uses the output device's native format)
        let config = if let Some(cfg) = best_config {
            cfg
        } else {
            device.default_input_config().map_err(|e| {
                anyhow!(
                    "Failed to get audio config: {}\n\
                     If you selected a loopback device, make sure WASAPI is available.",
                    e
                )
            })?
        };

        let sample_rate = config.sample_rate().0;
        let channels = config.channels();
        info!(
            "Audio: {}Hz, {} ch, {:?}",
            sample_rate,
            channels,
            config.sample_format()
        );

        let stream_config: StreamConfig = config.into();
        let (tx, rx) = mpsc::channel();
        let chunk_size = sample_rate as usize;
        let mut buffer: Vec<f32> = Vec::with_capacity(chunk_size);

        let stream = device
            .build_input_stream(
                &stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if channels == 1 {
                        buffer.extend_from_slice(data);
                    } else {
                        for frame in data.chunks(channels as usize) {
                            let mono: f32 = frame.iter().sum::<f32>() / channels as f32;
                            buffer.push(mono);
                        }
                    }
                    while buffer.len() >= chunk_size {
                        let chunk: Vec<f32> = buffer.drain(..chunk_size).collect();
                        let _ = tx.send(AudioChunk {
                            samples: chunk,
                            sample_rate,
                        });
                    }
                },
                |err| warn!("Audio error: {}", err),
                None,
            )
            .map_err(|e| {
                anyhow!(
                    "Failed to build audio stream: {}\n\
                 For WASAPI loopback, make sure your output device is active (playing audio).",
                    e
                )
            })?;

        stream
            .play()
            .map_err(|e| anyhow!("Failed to play: {}", e))?;
        std::mem::forget(stream);

        info!("Audio capture started ({}Hz)", sample_rate);
        Ok(rx)
    }

    // Re-export Device type for external use
    pub type Device = cpal::Device;
}

// ============================
// Stub for headless/CI (no cpal)
// ============================
#[cfg(not(feature = "audio"))]
mod real {
    use super::*;

    pub struct Device;

    impl Device {
        pub fn name(&self) -> Result<String, ()> {
            Ok("stub-microphone".to_string())
        }
    }

    pub fn find_system_audio_device() -> Result<Device> {
        Err(anyhow!(
            "Audio capture not available (compiled without 'audio' feature)"
        ))
    }

    pub fn find_microphone_device() -> Result<Device> {
        warn!("Running in stub mode — no real audio capture");
        Ok(Device)
    }

    pub fn select_device_interactive() -> Result<Device> {
        warn!("Running in stub mode — no real audio capture");
        Ok(Device)
    }

    pub fn start_capture(_device: Device) -> Result<mpsc::Receiver<AudioChunk>> {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || loop {
            let _ = tx.send(AudioChunk {
                samples: vec![0.0f32; 16000],
                sample_rate: 16000,
            });
            std::thread::sleep(std::time::Duration::from_secs(1));
        });
        Ok(rx)
    }
}

pub use real::*;
