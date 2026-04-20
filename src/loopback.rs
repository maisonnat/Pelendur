//! WASAPI Loopback Capture — System audio interception.
//!
//! Architecture follows Windows Core Audio best practices:
//!   1. COM MTA initialization (CoInitializeEx, COINIT_MULTITHREADED)
//!   2. Default render endpoint enumeration (IMMDeviceEnumerator)
//!   3. Mix format negotiation (IAudioClient::GetMixFormat — shared mode)
//!   4. Event-driven capture (AUDCLNT_STREAMFLAGS_EVENTCALLBACK + CreateEvent)
//!   5. MMCSS thread priority (AvSetMmThreadCharacteristics "Pro Audio")
//!   6. Graceful device invalidation recovery (AUDCLNT_E_DEVICE_INVALIDATED)
//!
//! Captures ALL audio playing through the default output device
//! (USB headset, Bluetooth, speakers).

use crate::audio::AudioChunk;
use anyhow::{anyhow, Result};
use std::sync::mpsc;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioProcess {
    pub pid: u32,
    pub name: String,
}

/// Process-level filtering not yet implemented.
/// Loopback captures the full system mix.
pub fn list_audio_processes() -> Vec<AudioProcess> {
    vec![]
}

// ═══════════════════════════════════════════════════════════════════════════
//  Windows Implementation
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(target_os = "windows")]
pub fn start_system_loopback_capture() -> Result<mpsc::Receiver<AudioChunk>> {
    let (tx, rx) = mpsc::channel();

    std::thread::Builder::new()
        .name("wasapi-loopback".into())
        .spawn(move || {
            if let Err(e) = wasapi_loopback_thread(tx) {
                eprintln!("  ❌ WASAPI loopback thread exited: {}", e);
            }
        })?;

    Ok(rx)
}

#[cfg(target_os = "windows")]
fn wasapi_loopback_thread(tx: mpsc::Sender<AudioChunk>) -> Result<()> {
    use windows::Win32::Media::Audio::*;
    use windows::Win32::System::Com::*;
    use windows::Win32::Foundation::*;
    use windows::core::GUID;

    // Raw FFI for functions not exposed by windows 0.58 features
    #[link(name = "kernel32")]
    extern "system" {
        fn CreateEventW(
            lpEventAttributes: *mut std::ffi::c_void,
            bManualReset: i32,
            bInitialState: i32,
            lpName: *const u16,
        ) -> isize; // HANDLE

        fn WaitForSingleObject(hHandle: isize, dwMilliseconds: u32) -> u32;

        fn CloseHandle(hObject: isize) -> i32;
    }

    #[link(name = "avrt")]
    extern "system" {
        fn AvSetMmThreadCharacteristicsW(
            TaskName: *const u16,
            TaskIndex: *mut u32,
        ) -> isize; // HANDLE

        fn AvRevertMmThreadCharacteristics(AvrtHandle: isize) -> i32;
    }

    const INFINITE: u32 = 0xFFFFFFFF;
    const WAIT_OBJECT_0: u32 = 0;
    const WAIT_TIMEOUT: u32 = 0x00000102;

    // ── Step 1: COM MTA initialization ──────────────────────────────────
    // Multi-Threaded Apartment avoids STA message-pump latency.
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
    }

    // ── Step 2: Enumerate default render endpoint ───────────────────────
    let enumerator: IMMDeviceEnumerator =
        unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };

    let device = unsafe {
        enumerator
            .GetDefaultAudioEndpoint(eRender, eConsole)
            .map_err(|e| anyhow!("No default render endpoint: {:?}", e))?
    };

    // ── Step 3: Activate IAudioClient + negotiate format ────────────────
    let audio_client: IAudioClient = unsafe { device.Activate(CLSCTX_ALL, None)? };

    // Get the audio engine's mix format (mandatory for shared mode)
    let mix_format = unsafe { audio_client.GetMixFormat()? };
    let pwfx = unsafe { &*mix_format };
    let sample_rate = pwfx.nSamplesPerSec;
    let channels = pwfx.nChannels;
    let block_align = pwfx.nBlockAlign as usize;
    let bits_per_sample = pwfx.wBitsPerSample as usize;
    let bytes_per_sample = bits_per_sample / 8;

    println!(
        "  🔊 WASAPI: {}Hz, {}ch, {}bit (mix format)",
        sample_rate, channels, bits_per_sample
    );

    // ── Step 4: Initialize with LOOPBACK + EVENTCALLBACK ────────────────
    const AUDCLNT_STREAMFLAGS_LOOPBACK: u32 = 0x00010000;
    const AUDCLNT_STREAMFLAGS_EVENTCALLBACK: u32 = 0x00040000;
    let stream_flags = AUDCLNT_STREAMFLAGS_LOOPBACK | AUDCLNT_STREAMFLAGS_EVENTCALLBACK;

    // Buffer = 1 second (in 100ns reftime units)
    let buffer_duration: i64 = 10_000_000;

    unsafe {
        audio_client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            stream_flags,
            buffer_duration,
            0,
            pwfx,
            None as Option<*const GUID>,
        )?
    };

    // ── Step 5: Create Win32 event + bind + get capture client ──────────
    let event_handle = unsafe { CreateEventW(std::ptr::null_mut(), 0, 0, std::ptr::null()) };
    if event_handle == 0 {
        return Err(anyhow!("CreateEventW failed"));
    }

    unsafe {
        audio_client.SetEventHandle(HANDLE(event_handle as *mut std::ffi::c_void))?;
    }

    let capture_client: IAudioCaptureClient = unsafe { audio_client.GetService()? };

    // ── Step 6: MMCSS "Pro Audio" thread priority ───────────────────────
    let mut task_index: u32 = 0;
    let pro_audio: Vec<u16> = "Pro Audio\0".encode_utf16().collect();
    let mmcss_handle =
        unsafe { AvSetMmThreadCharacteristicsW(pro_audio.as_ptr(), &mut task_index) };

    let mmcss_active = mmcss_handle != 0;
    if mmcss_active {
        println!("  🔊 WASAPI: MMCSS Pro Audio priority active");
    } else {
        println!("  ⚠️  WASAPI: MMCSS failed (non-fatal), continuing at normal priority");
    }

    // ── Step 7: Start capture loop ──────────────────────────────────────
    unsafe { audio_client.Start()? };
    println!("  🔊 WASAPI: Loopback capture started ✅");

    // Accumulate ~1 second of audio before sending a chunk
    let frames_per_chunk = sample_rate as usize;
    let mut pcm_buffer: Vec<u8> = Vec::with_capacity(block_align * frames_per_chunk);

    loop {
        // Sleep until the audio engine signals new data (event-driven)
        let wait_result = unsafe { WaitForSingleObject(event_handle, INFINITE) };

        if wait_result != WAIT_OBJECT_0 {
            if wait_result == WAIT_TIMEOUT {
                continue;
            }
            // Unexpected wait result — exit gracefully
            break;
        }

        // Read ALL available packets from the capture buffer
        loop {
            let mut data_ptr: *mut u8 = std::ptr::null_mut();
            let mut num_frames: u32 = 0;
            let mut flags: u32 = 0;

            let hr = unsafe {
                capture_client.GetBuffer(
                    &mut data_ptr,
                    &mut num_frames,
                    &mut flags,
                    None,
                    None,
                )
            };

            if let Err(e) = hr {
                let code = e.code().0 as u32;
                // AUDCLNT_S_BUFFER_EMPTY (0x088001) — no more packets, expected
                if code == 0x088001 {
                    break;
                }
                // AUDCLNT_E_DEVICE_INVALIDATED (0x88890004) — device disconnected
                if code == 0x88890004 {
                    eprintln!("  ⚠️  WASAPI: Device invalidated (disconnected?)");
                    let _ = tx.send(AudioChunk {
                        samples: vec![],
                        sample_rate,
                    });
                    break;
                }
                eprintln!("  ⚠️  WASAPI GetBuffer error: 0x{:08X}", code);
                break;
            }

            // Copy data from the capture buffer (unsafe slice construction)
            if !data_ptr.is_null() && num_frames > 0 {
                let byte_count = num_frames as usize * block_align;
                let slice = unsafe { std::slice::from_raw_parts(data_ptr, byte_count) };
                pcm_buffer.extend_from_slice(slice);
            }

            unsafe {
                let _ = capture_client.ReleaseBuffer(num_frames);
            }

            // Send chunks when we accumulated ~1 second of audio
            while pcm_buffer.len() >= block_align * frames_per_chunk {
                let chunk_bytes: Vec<u8> =
                    pcm_buffer.drain(..block_align * frames_per_chunk).collect();

                // Convert PCM bytes → f32 samples
                let samples: Vec<f32> = if bytes_per_sample == 4 {
                    // IEEE f32 (WASAPI shared mode default)
                    chunk_bytes
                        .chunks_exact(4)
                        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                        .collect()
                } else if bytes_per_sample == 2 {
                    // PCM 16-bit → normalize to [-1.0, 1.0]
                    chunk_bytes
                        .chunks_exact(2)
                        .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0)
                        .collect()
                } else {
                    continue;
                };

                // Mix down to mono
                let mono: Vec<f32> = if channels > 1 {
                    samples
                        .chunks(channels as usize)
                        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
                        .collect()
                } else {
                    samples
                };

                // Skip silence (RMS energy threshold)
                let energy: f32 = mono.iter().map(|s| s * s).sum::<f32>() / mono.len() as f32;
                if energy < 1e-6 {
                    continue;
                }

                if tx
                    .send(AudioChunk {
                        samples: mono,
                        sample_rate,
                    })
                    .is_err()
                {
                    // Receiver dropped — stop cleanly
                    unsafe {
                        let _ = audio_client.Stop();
                    }
                    if mmcss_active {
                        unsafe {
                            AvRevertMmThreadCharacteristics(mmcss_handle);
                        }
                    }
                    unsafe {
                        CloseHandle(event_handle);
                    }
                    println!("  🔊 WASAPI: Stopped (receiver dropped)");
                    return Ok(());
                }
            }
        }
    }

    // ── Step 8: Cleanup ─────────────────────────────────────────────────
    unsafe {
        let _ = audio_client.Stop();
    }
    if mmcss_active {
        unsafe {
            AvRevertMmThreadCharacteristics(mmcss_handle);
        }
    }
    unsafe {
        CloseHandle(event_handle);
    }
    println!("  🔊 WASAPI: Capture ended cleanly");
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
//  Linux stub
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(not(target_os = "windows"))]
pub fn start_system_loopback_capture() -> Result<mpsc::Receiver<AudioChunk>> {
    Err(anyhow!(
        "WASAPI loopback is Windows-only. Use PulseAudio/PipeWire on Linux."
    ))
}
