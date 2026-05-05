//! Dual Audio Capture Mixer — combines WASAPI loopback (system) + cpal mic.
//!
//! Architecture:
//!   1. Loopback bridge thread: reads from `loopback::start_system_loopback_capture()`
//!      and writes to a lock-free ring buffer.
//!   2. Mic bridge thread: reads from `audio::start_capture(device)`
//!      and writes to another ring buffer.
//!   3. Mixer core thread: reads from both ring buffers, applies
//!      `sample = (mic * 0.5) + (loopback * 0.5)`, sends to output channel.
//!
//! The callers must keep the returned `cpal::Stream` alive (same pattern as mic-only mode).
//! All three threads share the same lifetime — when the output Receiver is dropped,
//! the bridges' sends fail and the threads exit cleanly.

use crate::audio::AudioChunk;
use ringbuf::traits::{Consumer, Producer, Split};
use ringbuf::HeapRb;
use std::sync::mpsc;

/// Ring buffer capacity in AudioChunks (~32 seconds at 1 chunk/sec).
const RINGBUF_CAPACITY: usize = 32;

/// Start dual capture: system audio + microphone, mixed 50/50.
///
/// Takes a `cpal::Device` and returns a tuple of:
/// - `mpsc::Receiver<AudioChunk>` — mixed audio chunks for the STT pipeline
/// - `cpal::Stream` — must be kept alive (store it like any mic-only stream)
///
/// The returned receiver produces chunks synchronized to whichever source has data:
/// - Both available: 50/50 mixed, using loopback sample rate
/// - Only loopback: forwarded as-is
/// - Only mic: forwarded as-is
pub fn start_dual_capture(
    mic_device: impl Into<crate::audio::Device>,
) -> Result<(mpsc::Receiver<AudioChunk>, cpal::Stream), String> {
    let (tx, rx) = mpsc::channel();

    // ── Lock-free ring buffers (chunk-level, preserves sample_rate) ──────
    let loopback_rb = HeapRb::<AudioChunk>::new(RINGBUF_CAPACITY);
    let (mut loopback_prod, mut loopback_cons) = loopback_rb.split();

    let mic_rb = HeapRb::<AudioChunk>::new(RINGBUF_CAPACITY);
    let (mut mic_prod, mut mic_cons) = mic_rb.split();

    // ── Thread 1: Loopback → loopback ring buffer ────────────────────────
    let loopback_rx = crate::loopback::start_system_loopback_capture()
        .map_err(|e| format!("Failed to start loopback capture: {}", e))?;

    std::thread::Builder::new()
        .name("mixer-lb-bridge".into())
        .spawn(move || {
            while let Ok(chunk) = loopback_rx.recv() {
                // Non-blocking push; silently drop if ring buffer is full.
                let _ = loopback_prod.try_push(chunk);
            }
        })
        .map_err(|e| format!("Failed to spawn loopback bridge thread: {}", e))?;

    // ── Thread 2: Mic → mic ring buffer ──────────────────────────────────
    let (mic_rx, mic_stream) = crate::audio::start_capture(mic_device.into())
        .map_err(|e| format!("Failed to start mic capture: {}", e))?;

    std::thread::Builder::new()
        .name("mixer-mic-bridge".into())
        .spawn(move || {
            while let Ok(chunk) = mic_rx.recv() {
                let _ = mic_prod.try_push(chunk);
            }
        })
        .map_err(|e| format!("Failed to spawn mic bridge thread: {}", e))?;

    // ── Thread 3: Mixer core — accumulate mic until loopback arrives, then mix ───
    std::thread::Builder::new()
        .name("mixer-core".into())
        .spawn(move || {
            let mut mic_accum: Vec<f32> = Vec::new();
            loop {
                let lb_chunk = loopback_cons.try_pop();
                let mic_chunk = mic_cons.try_pop();
                
                // Accumulate any mic data
                if let Some(mic) = mic_chunk {
                    mic_accum.extend_from_slice(&mic.samples);
                }

                match lb_chunk {
                    Some(lb) => {
                        // Loopback data available — mix with accumulated mic
                        let mixed = if mic_accum.is_empty() {
                            lb.samples
                        } else {
                            // Truncate or pad mic_accum to match loopback length
                            if mic_accum.len() >= lb.samples.len() {
                                let (use_accum, rest) = mic_accum.split_at(lb.samples.len());
                                let mixed = mix_samples(&lb.samples, use_accum);
                                mic_accum = rest.to_vec();
                                mixed
                            } else {
                                // Pad with silence
                                let mut padded = mic_accum.clone();
                                padded.resize(lb.samples.len(), 0.0);
                                let mixed = mix_samples(&lb.samples, &padded);
                                mic_accum.clear();
                                mixed
                            }
                        };
                        if tx.send(AudioChunk {
                            samples: mixed,
                            sample_rate: lb.sample_rate,
                        }).is_err() { break; }
                    }
                    None => {
                        // Only mic data (no loopback yet) — keep accumulating
                        if mic_accum.len() > 48000 * 5 {
                            // If we have 5+ seconds of mic with no loopback,
                            // forward it as-is in chunks
                            let chunk: Vec<f32> = mic_accum.drain(..16000.min(mic_accum.len())).collect();
                            if tx.send(AudioChunk {
                                samples: chunk,
                                sample_rate: 16000,
                            }).is_err() { break; }
                        } else {
                            std::thread::sleep(std::time::Duration::from_millis(10));
                        }
                    }
                }
            }
        })
        .map_err(|e| format!("Failed to spawn mixer core thread: {}", e))?;

    Ok((rx, mic_stream))
}

/// Mix two audio sample buffers at a 50/50 ratio.
///
/// Uses the shorter buffer's length so we never panic on mismatched sizes
/// (which can happen if loopback and mic capture at different rates).
///
/// Formula: `output[i] = (a[i] * 0.5) + (b[i] * 0.5)`
fn mix_samples(a: &[f32], b: &[f32]) -> Vec<f32> {
    let len = a.len().min(b.len());
    a[..len]
        .iter()
        .zip(b[..len].iter())
        .map(|(sa, sb)| sa * 0.5 + sb * 0.5)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mix_samples_equal_length() {
        let a = vec![1.0, 0.0, 0.5, -0.5];
        let b = vec![0.0, 1.0, 0.5, -0.5];
        let result = mix_samples(&a, &b);
        assert_eq!(result.len(), 4);
        assert!((result[0] - 0.5).abs() < 1e-6);
        assert!((result[1] - 0.5).abs() < 1e-6);
        assert!((result[2] - 0.5).abs() < 1e-6);
        assert!((result[3] + 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_mix_samples_a_longer() {
        let a = vec![1.0, 0.5, 0.25, 0.125];
        let b = vec![0.0, 1.0];
        let result = mix_samples(&a, &b);
        assert_eq!(result.len(), 2);
        assert!((result[0] - 0.5).abs() < 1e-6);
        assert!((result[1] - 0.75).abs() < 1e-6);
    }

    #[test]
    fn test_mix_samples_b_longer() {
        let a = vec![0.8, 0.2];
        let b = vec![0.2, 0.8, 0.5];
        let result = mix_samples(&a, &b);
        assert_eq!(result.len(), 2);
        assert!((result[0] - 0.5).abs() < 1e-6);
        assert!((result[1] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_mix_samples_empty() {
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];
        let result = mix_samples(&a, &b);
        assert!(result.is_empty());
    }
}
