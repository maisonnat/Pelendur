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
pub mod real {
    use super::*;
    pub use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    pub use cpal::{SampleFormat, SampleRate, StreamConfig, Stream};

    pub fn find_system_audio_device() -> Result<Device> {
        let host = cpal::default_host();
        #[cfg(target_os = "windows")]
        { return find_system_audio_windows(&host); }
        #[cfg(target_os = "linux")]
        { return find_system_audio_linux(&host); }
        #[cfg(target_os = "macos")]
        { return find_system_audio_macos(&host); }
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        { Err(anyhow!("System audio capture not supported on this platform")) }
    }

    #[cfg(target_os = "windows")]
    fn find_system_audio_windows(host: &cpal::Host) -> Result<Device> {
        let devices = host.input_devices().map_err(|e| anyhow!("Failed to enumerate input devices: {}", e))?;
        let mut all_devices = Vec::new();
        for device in devices {
            let name = device.name().unwrap_or_default();
            all_devices.push((name.clone(), device));
            if name.to_lowercase().contains("loopback") {
                return Ok(all_devices.pop().unwrap().1);
            }
        }
        if let Some((_, device)) = all_devices.pop() {
            return Ok(device);
        }
        Err(anyhow!("No audio input device found"))
    }

    #[cfg(target_os = "linux")]
    fn find_system_audio_linux(host: &cpal::Host) -> Result<Device> {
        let devices = host.input_devices().map_err(|e| anyhow!("Failed to enumerate input devices: {}", e))?;
        for device in devices {
            let name = device.name().unwrap_or_default();
            if name.to_lowercase().contains("monitor") { return Ok(device); }
        }
        Err(anyhow!("No system audio monitor device found"))
    }

    #[cfg(target_os = "macos")]
    fn find_system_audio_macos(host: &cpal::Host) -> Result<Device> {
        let devices = host.input_devices().map_err(|e| anyhow!("Failed to enumerate input devices: {}", e))?;
        for device in devices {
            let name = device.name().unwrap_or_default();
            if name.to_lowercase().contains("blackhole") { return Ok(device); }
        }
        host.default_input_device().ok_or_else(|| anyhow!("No input device found"))
    }

    pub fn find_microphone_device() -> Result<Device> {
        let host = cpal::default_host();
        host.default_input_device().ok_or_else(|| anyhow!("No default input device found"))
    }

    fn enumerate_input_devices() -> Result<Vec<(String, Device, bool, String)>> {
        let host = cpal::default_host();
        let default_input_name = host.default_input_device().and_then(|d| d.name().ok()).unwrap_or_default();
        let default_output_name = host.default_output_device().and_then(|d| d.name().ok()).unwrap_or_default();
        let mut result = Vec::new();

        if let Ok(output_devices) = host.output_devices() {
            for device in output_devices {
                let name = device.name().unwrap_or_default();
                let lower = name.to_lowercase();
                let is_default = name == default_output_name;
                let label = if lower.contains("speakers") || lower.contains("altavoces") { "🔊 Loopback (speakers)" } else { "🔊 Loopback (output)" };
                result.push((name, device, is_default, label.to_string()));
            }
        }

        if let Ok(input_devices) = host.input_devices() {
            for device in input_devices {
                let name = device.name().unwrap_or_default();
                let lower = name.to_lowercase();
                let is_default = name == default_input_name;
                let label = if lower.contains("micro") || lower.contains("mic") { "🎤 Microphone" } else { "🎤 Input" };
                result.push((name, device, is_default, label.to_string()));
            }
        }
        Ok(result)
    }

    pub fn select_device_interactive() -> Result<Device> {
        let devices = enumerate_input_devices()?;
        if devices.is_empty() {
            anyhow::bail!("No audio input devices found. Check your microphone connection.");
        }
        for (i, (name, _, is_default, label)) in devices.iter().enumerate() {
            let marker = if *is_default { " ← default" } else { "" };
            println!("    [{}] {} {}{}", i + 1, label, name, marker);
        }
        print!("  Select [1-{}]: ", devices.len());
        use std::io::Write;
        std::io::stdout().flush().ok();
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
        let idx: usize = input.trim().parse().unwrap_or(1);
        Ok(devices.get(idx - 1).map(|d| d.1.clone()).unwrap_or(devices[0].1.clone()))
    }

    pub fn start_capture(device: Device) -> Result<(mpsc::Receiver<AudioChunk>, cpal::Stream)> {
        println!("  ⚙ Intentando abrir dispositivo...");
        let config = device.default_input_config()?;
        let sample_rate = config.sample_rate().0;
        let channels = config.channels();
        let sample_format = config.sample_format();
        println!("  ✓ Formato: {}Hz, {} canales, {:?}", sample_rate, channels, sample_format);

        let stream_config: StreamConfig = config.into();
        let (tx, rx) = mpsc::channel();
        let mut buffer: Vec<f32> = Vec::with_capacity(sample_rate as usize * 2);

        let error_callback = |err| eprintln!("  ❌ Error audio: {}", err);

        let stream = match sample_format {
            SampleFormat::F32 => device.build_input_stream(&stream_config, move |data: &[f32], _| process_samples(data, channels, &mut buffer, &tx, sample_rate), error_callback, None)?,
            SampleFormat::I16 => device.build_input_stream(&stream_config, move |data: &[i16], _| {
                let f32_data: Vec<f32> = data.iter().map(|&s| s as f32 / 32768.0).collect();
                process_samples(&f32_data, channels, &mut buffer, &tx, sample_rate)
            }, error_callback, None)?,
            SampleFormat::U16 => device.build_input_stream(&stream_config, move |data: &[u16], _| {
                let f32_data: Vec<f32> = data.iter().map(|&s| (s as f32 - 32768.0) / 32768.0).collect();
                process_samples(&f32_data, channels, &mut buffer, &tx, sample_rate)
            }, error_callback, None)?,
            _ => return Err(anyhow!("Unsupported format")),
        };

        stream.play()?;
        println!("  🚀 Stream activo.");
        Ok((rx, stream))
    }

    fn process_samples(data: &[f32], channels: u16, buffer: &mut Vec<f32>, tx: &mpsc::Sender<AudioChunk>, sample_rate: u32) {
        if channels == 1 {
            buffer.extend_from_slice(data);
        } else {
            for frame in data.chunks(channels as usize) {
                buffer.push(frame.iter().sum::<f32>() / channels as f32);
            }
        }
        let chunk_size = sample_rate as usize;
        while buffer.len() >= chunk_size {
            let chunk: Vec<f32> = buffer.drain(..chunk_size).collect();
            let _ = tx.send(AudioChunk { samples: chunk, sample_rate });
        }
    }

    pub type Device = cpal::Device;
}

#[cfg(not(feature = "audio"))]
mod real {
    use super::*;
    pub struct Device;
    pub fn find_system_audio_device() -> Result<Device> { Err(anyhow!("No audio")) }
    pub fn find_microphone_device() -> Result<Device> { Ok(Device) }
    pub fn select_device_interactive() -> Result<Device> { Ok(Device) }
    pub fn start_capture(_d: Device) -> Result<(mpsc::Receiver<AudioChunk>, ())> { Err(anyhow!("No audio")) }
}

pub use real::*;
