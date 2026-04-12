//! Per-application audio loopback capture using WASAPI.
//!
//! Captures audio from a specific process (Zoom, Chrome, etc.)
//! using Windows WASAPI Process Loopback mode.

#[cfg(feature = "wasapi")]
pub mod real {
    use crate::audio::AudioChunk;
    use anyhow::{anyhow, Result};
    use std::collections::VecDeque;
    use std::sync::mpsc;
    use sysinfo::System;
    use tracing::{debug, info, warn};
    use wasapi::*;

    /// Information about a process that might be producing audio.
    #[derive(Debug, Clone)]
    pub struct AudioProcess {
        pub pid: u32,
        pub name: String,
    }

    /// List processes that might be producing audio.
    /// Returns all running processes (we can't easily detect which ones have audio,
    /// so we filter by common audio app names).
    pub fn list_audio_processes() -> Vec<AudioProcess> {
        let mut sys = System::new_all();
        sys.refresh_all();

        let audio_keywords = [
            "zoom", "chrome", "firefox", "teams", "slack", "discord", "spotify", "youtube", "edge",
            "opera", "brave", "vlc", "mpv", "obs", "skype", "telegram", "whatsapp", "notion",
        ];

        let mut processes: Vec<AudioProcess> = Vec::new();
        for (pid, process) in sys.processes() {
            let name = process.name().to_string_lossy().to_lowercase();
            if audio_keywords.iter().any(|kw| name.contains(kw)) {
                processes.push(AudioProcess {
                    pid: pid.as_u32(),
                    name: process.name().to_string_lossy().to_string(),
                });
            }
        }

        // If no known audio apps found, show all processes with PIDs > 1000
        if processes.is_empty() {
            for (pid, process) in sys.processes() {
                let pid_val = pid.as_u32();
                if pid_val > 1000 && !process.name().to_string_lossy().contains("System") {
                    processes.push(AudioProcess {
                        pid: pid_val,
                        name: process.name().to_string_lossy().to_string(),
                    });
                }
            }
            processes.truncate(20); // Limit to 20
        }

        processes.sort_by(|a, b| a.name.cmp(&b.name));
        processes
    }

    /// Interactive selector for audio processes.
    pub fn select_audio_process() -> Result<AudioProcess> {
        let processes = list_audio_processes();

        if processes.is_empty() {
            return Err(anyhow!("No audio processes found"));
        }

        println!("  Active audio applications:");
        println!();

        for (i, proc) in processes.iter().enumerate() {
            println!("    [{}] {} (PID: {})", i + 1, proc.name, proc.pid);
        }

        println!();
        print!("  Select app [1-{}]: ", processes.len());
        use std::io::Write;
        std::io::stdout().flush().ok();

        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
        let input = input.trim();

        let idx: usize = input.parse().map_err(|_| anyhow!("Invalid selection"))?;
        if idx < 1 || idx > processes.len() {
            return Err(anyhow!("Selection out of range"));
        }

        let selected = processes[idx - 1].clone();
        println!("  ✓ Selected: {} (PID: {})", selected.name, selected.pid);
        println!();

        Ok(selected)
    }

    /// Start loopback capture for a specific process.
    /// Returns a receiver that yields AudioChunk frames.
    /// Runs the capture loop on a separate thread (AudioClient is !Send).
    pub fn start_loopback_capture(
        process_id: u32,
        include_tree: bool,
    ) -> Result<mpsc::Receiver<AudioChunk>> {
        // Try to get the mix format from the loopback client
        // We need to initialize COM on the capture thread
        let (tx, rx) = mpsc::channel();

        std::thread::Builder::new()
            .name("loopback-capture".to_string())
            .spawn(move || {
                if let Err(e) = capture_thread(process_id, include_tree, tx) {
                    warn!("Loopback capture error: {}", e);
                }
            })?;

        Ok(rx)
    }

    fn capture_thread(
        process_id: u32,
        include_tree: bool,
        tx: mpsc::Sender<AudioChunk>,
    ) -> Result<()> {
        initialize_mta().ok(); // Don't fail on COM init, may already be initialized

        info!(
            "Starting loopback capture for PID {} (include_tree: {})",
            process_id, include_tree
        );

        let mut audio_client =
            AudioClient::new_application_loopback_client(process_id, include_tree)
                .map_err(|e| anyhow!("Failed to create loopback client: {:?}", e))?;

        // NOTE: get_mixformat() returns E_NOTIMPL for application loopback clients.
        // Use a standard format and let autoconvert handle the conversion.
        let sample_rate: usize = 44100;
        let channels: usize = 2;

        let desired_format =
            WaveFormat::new(32, 32, &SampleType::Float, sample_rate, channels, None);

        let buffer_duration_hns = 200_000; // 20ms

        let mode = StreamMode::EventsShared {
            autoconvert: true,
            buffer_duration_hns,
        };

        audio_client
            .initialize_client(&desired_format, &Direction::Capture, &mode)
            .map_err(|e| anyhow!("Failed to initialize loopback client: {:?}", e))?;

        info!("Loopback capture: {}Hz, {} ch", sample_rate, channels);

        let h_event = audio_client
            .set_get_eventhandle()
            .map_err(|e| anyhow!("Failed to get event handle: {:?}", e))?;

        let capture_client = audio_client
            .get_audiocaptureclient()
            .map_err(|e| anyhow!("Failed to get capture client: {:?}", e))?;

        audio_client
            .start_stream()
            .map_err(|e| anyhow!("Failed to start stream: {:?}", e))?;

        let blockalign = desired_format.get_blockalign() as usize;
        let mut sample_queue: VecDeque<u8> = VecDeque::with_capacity(
            blockalign * sample_rate as usize * 2, // 2 seconds buffer
        );

        let chunk_size = sample_rate as usize; // 1 second chunks
        let channels_u16 = channels;

        loop {
            // Read available data
            capture_client
                .read_from_device_to_deque(&mut sample_queue)
                .map_err(|e| anyhow!("Failed to read from device: {:?}", e))?;

            // Convert and send chunks
            let bytes_per_chunk = blockalign * chunk_size;
            while sample_queue.len() >= bytes_per_chunk {
                let mut chunk_bytes = vec![0u8; bytes_per_chunk];
                for byte in chunk_bytes.iter_mut() {
                    *byte = sample_queue.pop_front().unwrap();
                }

                // Convert bytes to f32 samples
                let samples: Vec<f32> = chunk_bytes
                    .chunks_exact(4) // 32-bit float = 4 bytes
                    .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                    .collect();

                // Mix to mono if stereo
                let mono_samples = if channels_u16 > 1 {
                    samples
                        .chunks(channels_u16 as usize)
                        .map(|frame| frame.iter().sum::<f32>() / channels_u16 as f32)
                        .collect()
                } else {
                    samples
                };

                let _ = tx.send(AudioChunk {
                    samples: mono_samples,
                    sample_rate: sample_rate as u32,
                });
            }

            // Wait for next buffer
            if h_event.wait_for_event(1000).is_err() {
                warn!("Loopback capture event timeout, stopping");
                audio_client
                    .stop_stream()
                    .map_err(|e| anyhow!("Failed to stop stream: {:?}", e))?;
                break;
            }
        }

        info!("Loopback capture thread ended");
        Ok(())
    }
}

// Stub when wasapi is not available
#[cfg(not(feature = "wasapi"))]
pub mod real {
    use anyhow::{anyhow, Result};
    use std::sync::mpsc;

    #[derive(Debug, Clone)]
    pub struct AudioProcess {
        pub pid: u32,
        pub name: String,
    }

    pub fn list_audio_processes() -> Vec<AudioProcess> {
        vec![]
    }

    pub fn select_audio_process() -> Result<AudioProcess> {
        Err(anyhow!(
            "WASAPI loopback not available (feature 'wasapi' disabled)"
        ))
    }

    pub fn start_loopback_capture(
        _process_id: u32,
        _include_tree: bool,
    ) -> Result<mpsc::Receiver<crate::audio::AudioChunk>> {
        Err(anyhow!(
            "WASAPI loopback not available (feature 'wasapi' disabled)"
        ))
    }
}
