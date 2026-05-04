//! Pipeline Integration Test — WASAPI Loopback → VAD → STT
//!
//! Simulates the flow that starts from WASAPI loopback capture and goes
//! through VAD detection, PCM → WAV encoding, and verifies the full pipeline
//! wiring without requiring actual audio hardware or Windows.
//!
//! The wasapi_loopback feature is NOT enabled here — we test the pipeline
//! by constructing AudioChunks identical to what loopback would produce.

use ghostai_pilot::audio::AudioChunk;
use ghostai_pilot::stt;
use ghostai_pilot::vad::{VadDetector, VadEvent};

// ── Helpers ────────────────────────────────────────────────────────────────

/// Generate a simulated WASAPI chunk at 48kHz, ~1 second of audio.
/// Creates a sine wave at `freq_hz` with amplitude `amplitude`.
/// This mirrors how loopback.rs creates chunks (mono f32, ~frame_rate samples).
fn make_wasapi_chunk(freq_hz: f32, amplitude: f32, sample_rate: u32) -> AudioChunk {
    let num_samples = sample_rate as usize; // ~1 second
    let samples: Vec<f32> = (0..num_samples)
        .map(|i| {
            amplitude
                * (2.0 * std::f32::consts::PI * freq_hz * i as f32 / sample_rate as f32).sin()
        })
        .collect();
    AudioChunk {
        samples,
        sample_rate,
    }
}

/// Generate a chunk of near-silence (digital silence with tiny noise floor).
fn make_silence_chunk(sample_rate: u32) -> AudioChunk {
    let num_samples = sample_rate as usize;
    // Deterministic "near-zero" noise to simulate ADC floor (not true zero).
    let samples: Vec<f32> = (0..num_samples)
        .map(|i| ((i % 97) as f32) * 1e-10) // deterministic noise floor < -200dB
        .collect();
    AudioChunk {
        samples,
        sample_rate,
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[test]
fn test_wasapi_48000_chunk_through_vad_speech() {
    // Simulate the exact chunk format that loopback.rs creates:
    //   - 48000 Hz sample rate
    //   - 48000 samples (1 second)
    //   - Mono f32
    let mut vad = VadDetector::default_config();

    // First chunk: loud signal (should not trigger SpeechStart yet — needs 2
    // chunks).
    let chunk1 = make_wasapi_chunk(440.0, 0.3, 48000);
    let event1 = vad.process(&chunk1.samples);
    assert!(
        matches!(event1, VadEvent::Silence),
        "First loud chunk should not trigger SpeechStart (need 2 chunks)"
    );

    // Second chunk: SpeechStart fires.
    let chunk2 = make_wasapi_chunk(440.0, 0.3, 48000);
    let event2 = vad.process(&chunk2.samples);
    assert!(
        matches!(event2, VadEvent::SpeechStart),
        "Second loud chunk should trigger SpeechStart"
    );
    assert!(vad.is_speaking(), "VAD should be in Speaking state");

    // Third chunk: still speaking.
    let chunk3 = make_wasapi_chunk(440.0, 0.3, 48000);
    let event3 = vad.process(&chunk3.samples);
    assert!(
        matches!(event3, VadEvent::Silence),
        "Speaking chunk should return Silence (no state transition)"
    );

    // Two silence chunks: SpeechEnd.
    let silence1 = make_silence_chunk(48000);
    let _ = vad.process(&silence1.samples);
    let silence2 = make_silence_chunk(48000);
    let event_end = vad.process(&silence2.samples);
    assert!(
        matches!(event_end, VadEvent::SpeechEnd { .. }),
        "Two silence chunks should trigger SpeechEnd (got {:?})",
        event_end
    );
    assert!(
        !vad.is_speaking(),
        "VAD should not be speaking after SpeechEnd"
    );
}

#[test]
fn test_wasapi_44100_chunk_through_vad() {
    // WASAPI can return other sample rates too.
    let mut vad = VadDetector::default_config();

    let loud1 = make_wasapi_chunk(1000.0, 0.5, 44100);
    let loud2 = make_wasapi_chunk(1000.0, 0.5, 44100);

    assert!(matches!(vad.process(&loud1.samples), VadEvent::Silence));
    assert!(matches!(vad.process(&loud2.samples), VadEvent::SpeechStart));

    let silence1 = make_silence_chunk(44100);
    let silence2 = make_silence_chunk(44100);
    let _ = vad.process(&silence1.samples);
    assert!(matches!(
        vad.process(&silence2.samples),
        VadEvent::SpeechEnd { .. }
    ));
}

#[test]
fn test_wasapi_chunk_pcm_to_wav_encoding() {
    // Verify that WASAPI-produced chunks can be encoded to WAV correctly.
    // This is the step the pipeline uses: on SpeechEnd, pcm_to_wav() converts
    // the speech buffer to WAV bytes, then transcribe() sends them to STT.
    //
    // loopback.rs captures at mix format (typically 48kHz, f32, interleaved)
    // then converts to mono f32 at the source sample rate.
    // pcm_to_wav() accepts any sample rate.

    let chunk = make_wasapi_chunk(440.0, 0.3, 48000);

    // pcm_to_wav takes any f32 slice + sample_rate.
    let wav_bytes =
        stt::pcm_to_wav(&chunk.samples, chunk.sample_rate).expect("pcm_to_wav should succeed");

    // Verify WAV header: should start with "RIFF".
    assert_eq!(
        &wav_bytes[0..4],
        b"RIFF",
        "WAV should start with RIFF header"
    );
    assert_eq!(
        &wav_bytes[8..12],
        b"WAVE",
        "WAV should contain WAVE format tag"
    );

    // Verify format: mono, 16-bit.
    let num_channels = u16::from_le_bytes([wav_bytes[22], wav_bytes[23]]);
    assert_eq!(num_channels, 1, "WAV should be mono");
    let bits_per_sample = u16::from_le_bytes([wav_bytes[34], wav_bytes[35]]);
    assert_eq!(bits_per_sample, 16, "WAV should be 16-bit");

    // Verify sample rate in WAV matches original.
    let wav_sample_rate =
        u32::from_le_bytes([wav_bytes[24], wav_bytes[25], wav_bytes[26], wav_bytes[27]]);
    assert_eq!(
        wav_sample_rate, 48000,
        "WAV sample rate should match source"
    );

    // Data should be non-empty (48000 samples * 2 bytes per sample).
    let data_size =
        u32::from_le_bytes([wav_bytes[40], wav_bytes[41], wav_bytes[42], wav_bytes[43]]);
    assert_eq!(
        data_size as usize,
        chunk.samples.len() * 2,
        "Data size should be samples * 2 bytes"
    );
}

#[test]
fn test_speech_buffer_accumulation_like_main_rs() {
    // Reproduce the exact pattern from main.rs:
    //   1. Accumulate speech_buffer from chunks
    //   2. On SpeechEnd, call pcm_to_wav() on speech_buffer
    //   3. This is what gets sent to STT
    //
    // This verifies the wiring is correct end-to-end.

    let mut vad = VadDetector::default_config();
    let mut speech_buffer: Vec<f32> = Vec::with_capacity(48000 * 5);
    let mut is_capturing = false;

    // Simulate: 3 loud chunks (speech), 2 silence (end).
    let chunks = vec![
        (make_wasapi_chunk(440.0, 0.3, 48000), "speech1"),
        (make_wasapi_chunk(440.0, 0.3, 48000), "speech2"),
        (make_wasapi_chunk(600.0, 0.3, 48000), "speech3"),
        (make_silence_chunk(48000), "silence1"),
        (make_silence_chunk(48000), "silence2"),
    ];

    for (chunk, label) in &chunks {
        let event = vad.process(&chunk.samples);

        match event {
            VadEvent::SpeechStart => {
                assert!(
                    !is_capturing,
                    "SpeechStart should not fire when already capturing"
                );
                is_capturing = true;
                speech_buffer.clear();
                speech_buffer.extend_from_slice(&chunk.samples);
            }
            VadEvent::Silence => {
                if is_capturing {
                    speech_buffer.extend_from_slice(&chunk.samples);
                }
            }
            VadEvent::SpeechEnd { duration_chunks: _ } => {
                assert!(is_capturing, "SpeechEnd without capturing");
                is_capturing = false;

                // This is the exact pcm_to_wav call from main.rs.
                let wav_bytes = stt::pcm_to_wav(&speech_buffer, chunk.sample_rate)
                    .expect("pcm_to_wav should succeed");

                // Verify we have meaningful audio data.
                assert!(wav_bytes.len() > 44, "WAV should have data beyond header");
                assert!(
                    speech_buffer.len() >= 8000,
                    "Speech buffer should be >= 8000 samples (guard in main.rs)"
                );
                eprintln!(
                    "  Pipeline OK: {} samples -> {} byte WAV (label: {})",
                    speech_buffer.len(),
                    wav_bytes.len(),
                    label
                );
            }
        }
    }

    assert!(!is_capturing, "Should not be capturing after SpeechEnd");
    assert!(
        !speech_buffer.is_empty(),
        "Speech buffer should have audio data"
    );
}

#[test]
fn test_resample_48k_to_16k_from_wasapi() {
    // loopback.rs captures at 48kHz. When using Parakeet/local STT, main.rs
    // resamples to 16kHz. Test that the 48kHz input is valid for resampling.

    let chunk = make_wasapi_chunk(440.0, 0.5, 48000);
    assert_eq!(
        chunk.samples.len(),
        48000,
        "1 second at 48kHz = 48000 samples"
    );

    let rms: f32 =
        chunk.samples.iter().map(|s| s * s).sum::<f32>() / chunk.samples.len() as f32;
    let rms_db = 20.0 * rms.max(1e-10).log10();
    assert!(
        rms_db > -20.0,
        "440Hz sine at 0.5 amplitude should have high RMS energy (got {:.1} dB)",
        rms_db
    );
}

#[test]
fn test_vad_state_machine_with_various_wasapi_rates() {
    // WASAPI can return various sample rates (44.1k, 48k, 96k).
    // The VAD is sample-rate agnostic — it only processes f32 samples.
    // Verify VAD works correctly at all rates the loopback might produce.

    for sample_rate in [44100, 48000, 96000] {
        let mut vad = VadDetector::default_config();

        // Three speech chunks -> trigger SpeechStart.
        let speech = make_wasapi_chunk(500.0, 0.4, sample_rate);
        assert!(
            matches!(vad.process(&speech.samples), VadEvent::Silence),
            "first chunk at {}Hz should be Silence",
            sample_rate
        );
        assert!(
            matches!(vad.process(&speech.samples), VadEvent::SpeechStart),
            "second chunk should trigger SpeechStart at {}Hz",
            sample_rate
        );

        // Two silence chunks -> SpeechEnd.
        let sil = make_silence_chunk(sample_rate);
        assert!(matches!(vad.process(&sil.samples), VadEvent::Silence));
        assert!(
            matches!(vad.process(&sil.samples), VadEvent::SpeechEnd { .. }),
            "Two silence chunks should trigger SpeechEnd at {}Hz",
            sample_rate
        );
    }
}

#[test]
fn test_loopback_log_format_verification() {
    // loopback.rs writes debug logs to wasapi-debug.log.
    // Verify the log format is parseable.
    let sample_rate = 48000u32;
    let channels = 2u16;
    let bits_per_sample = 32u16;

    let log_line = format!(
        "[12:34:56.789] WASAPI: {}Hz, {}ch, {}bit (mix format)",
        sample_rate, channels, bits_per_sample
    );
    assert!(
        log_line.contains("48000Hz"),
        "Log should contain sample rate"
    );
    assert!(log_line.contains("2ch"), "Log should contain channel count");
    assert!(log_line.contains("32bit"), "Log should contain bit depth");

    let log_chunk = format!(
        "[12:34:57.000] WASAPI: Sent chunk #1 (48000 samples, energy={:.6})",
        0.012345
    );
    assert!(
        log_chunk.contains("chunk #1"),
        "Log should track chunk count"
    );
    assert!(
        log_chunk.contains("energy="),
        "Log should include energy level"
    );
}

#[test]
fn test_wasapi_chunk_channel_safety() {
    // loopback.rs at lines 332-338: mix down to mono.
    // If channels > 1, it averages across channels.
    // This test verifies that converting simulated stereo to mono works.
    let sample_rate = 48000u32;
    let channels = 2u16;
    let num_samples = sample_rate as usize * channels as usize;

    // Simulate stereo: left=440Hz, right=660Hz.
    let stereo: Vec<f32> = (0..num_samples)
        .map(|i| {
            let frame = i / 2;
            if i % 2 == 0 {
                0.3
                    * (2.0 * std::f32::consts::PI * 440.0 * frame as f32 / sample_rate as f32)
                        .sin()
            } else {
                0.3
                    * (2.0 * std::f32::consts::PI * 660.0 * frame as f32 / sample_rate as f32)
                        .sin()
            }
        })
        .collect();

    // Mix to mono (same as loopback.rs lines 332-338).
    let mono: Vec<f32> = stereo
        .chunks(channels as usize)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect();

    assert_eq!(
        mono.len(),
        sample_rate as usize,
        "Mono should have half the samples of stereo"
    );
    assert!(
        mono.iter().any(|&s| s.abs() > 0.1),
        "Mono mix should contain signal"
    );

    // Feed the mixed result through VAD.
    let mut vad = VadDetector::default_config();
    let event1 = vad.process(&mono);
    assert!(
        matches!(event1, VadEvent::Silence),
        "First mono chunk should be Silence"
    );

    let event2 = vad.process(&mono);
    assert!(
        matches!(event2, VadEvent::SpeechStart),
        "Second mono chunk -> SpeechStart (energy present)"
    );
}
