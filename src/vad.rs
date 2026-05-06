use tracing::debug;

/// Simple energy-based Voice Activity Detection.
/// Detects when someone is speaking based on audio energy.
pub struct VadDetector {
    /// RMS threshold in dB (e.g., -30.0)
    threshold_db: f32,
    /// Minimum duration of speech to trigger (in chunks, ~1 sec each)
    min_speech_chunks: usize,
    /// Minimum silence duration to end speech (in chunks)
    min_silence_chunks: usize,
    /// Internal state
    state: VadState,
    speech_chunk_count: usize,
    silence_chunk_count: usize,
}

#[derive(Debug, PartialEq)]
pub enum VadState {
    Silence,
    Speaking,
}

#[derive(Debug)]
pub enum VadEvent {
    /// Speech detected, audio samples included
    SpeechStart,
    /// Speech ended, accumulated audio included
    SpeechEnd { duration_chunks: usize },
    /// No speech
    Silence,
}

impl VadDetector {
    pub fn new(threshold_db: f32, min_speech_chunks: usize, min_silence_chunks: usize) -> Self {
        Self {
            threshold_db,
            min_speech_chunks,
            min_silence_chunks,
            state: VadState::Silence,
            speech_chunk_count: 0,
            silence_chunk_count: 0,
        }
    }

    /// Default config: -35dB threshold (balanced), 2 chunks min speech, 2 chunks min silence
    pub fn default_config() -> Self {
        Self::new(-35.0, 2, 2)
    }

    /// Process a chunk of audio samples. Returns a VadEvent.
    pub fn process(&mut self, samples: &[f32]) -> VadEvent {
        let rms = calculate_rms(samples);
        let rms_db = 20.0 * rms.max(1e-10).log10();

        let is_loud = rms_db > self.threshold_db;

        match self.state {
            VadState::Silence => {
                if is_loud {
                    self.speech_chunk_count += 1;
                    if self.speech_chunk_count >= self.min_speech_chunks {
                        self.state = VadState::Speaking;
                        self.silence_chunk_count = 0;
                        debug!("VAD: Speech started (rms_db={:.1})", rms_db);
                        return VadEvent::SpeechStart;
                    }
                } else {
                    self.speech_chunk_count = 0;
                }
                VadEvent::Silence
            }
            VadState::Speaking => {
                if is_loud {
                    self.silence_chunk_count = 0;
                } else {
                    self.silence_chunk_count += 1;
                    if self.silence_chunk_count >= self.min_silence_chunks {
                        let duration = self.speech_chunk_count;
                        self.state = VadState::Silence;
                        self.speech_chunk_count = 0;
                        self.silence_chunk_count = 0;
                        debug!("VAD: Speech ended ({} chunks)", duration);
                        return VadEvent::SpeechEnd {
                            duration_chunks: duration,
                        };
                    }
                }
                // Still speaking or brief pause — don't emit event
                // (the caller accumulates audio while in Speaking state)
                VadEvent::Silence
            }
        }
    }

    pub fn is_speaking(&self) -> bool {
        self.state == VadState::Speaking
    }

    pub fn reset(&mut self) {
        self.state = VadState::Silence;
        self.speech_chunk_count = 0;
        self.silence_chunk_count = 0;
    }
}

fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = samples.iter().map(|s| s * s).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

// ─── WebRTC VAD (ML-based, superior to energy-based) ──────────

/// WebRTC-based Voice Activity Detection using the full-rank VAD model.
///
/// Uses the `webrtc-vad` crate (Chromium's VAD algorithm) for ML-based
/// voice detection. Significantly more accurate than energy-based VAD:
/// - Better noise rejection (adaptive threshold)
/// - Works across sample rates (8k-48k)
/// - Lower false-positive rate in noisy environments
///
/// Maintains an internal sample buffer to handle arbitrary chunk sizes.
pub struct WebRtcVadDetector {
    vad: webrtc_vad::Vad,
    /// Internal buffer to accumulate samples until we have a full frame
    sample_buf: Vec<f32>,
    /// Frame size in samples (30ms @ configured rate)
    frame_size: usize,
    /// State machine
    state: VadState,
    /// Consecutive speech frames counted so far
    speech_frames: usize,
    /// Consecutive silence frames during speech
    silence_frames: usize,
    /// Min speech frames to trigger SpeechStart
    min_speech_frames: usize,
    /// Min silence frames to trigger SpeechEnd (hangover)
    min_silence_frames: usize,
    /// Actual sample rate for calculating RMS fallback
    sample_rate: u32,
}

impl WebRtcVadDetector {
    /// Create a new WebRTC VAD detector.
    ///
    /// * `sample_rate` - Audio sample rate (Hz). Internally uses 16kHz for VAD.
    /// * `min_speech_ms` - Minimum speech duration to trigger (ms)
    /// * `min_silence_ms` - Minimum silence to end speech / hangover (ms)
    pub fn new(sample_rate: u32, min_speech_ms: u32, min_silence_ms: u32) -> Self {
        let rate = match sample_rate {
            8000 => webrtc_vad::SampleRate::Rate8kHz,
            16000 => webrtc_vad::SampleRate::Rate16kHz,
            32000 => webrtc_vad::SampleRate::Rate32kHz,
            48000 => webrtc_vad::SampleRate::Rate48kHz,
            _ => webrtc_vad::SampleRate::Rate16kHz,
        };
        // 30ms frames. Shorter = more responsive but less stable.
        let frame_ms: u32 = 30;
        let frame_size = (sample_rate as usize * frame_ms as usize) / 1000;
        let mut vad = webrtc_vad::Vad::new_with_rate(rate);
        // Aggressiveness: 0=quality, 1=low bitrate, 2=aggressive, 3=very aggressive
        vad.set_mode(webrtc_vad::VadMode::Aggressive);

        Self {
            vad,
            sample_buf: Vec::with_capacity(frame_size),
            frame_size,
            state: VadState::Silence,
            speech_frames: 0,
            silence_frames: 0,
            min_speech_frames: (min_speech_ms / frame_ms).max(1) as usize,
            min_silence_frames: (min_silence_ms / frame_ms).max(1) as usize,
            sample_rate,
        }
    }

    /// Default: 16kHz, 100ms min speech, 500ms min silence.
    pub fn default_config() -> Self {
        Self::new(16000, 100, 500)
    }

    /// Process a chunk of audio samples. Returns a VadEvent.
    pub fn process(&mut self, samples: &[f32]) -> VadEvent {
        // Accumulate samples until we have enough for one frame
        self.sample_buf.extend_from_slice(samples);

        if self.sample_buf.len() < self.frame_size {
            return VadEvent::Silence;
        }

        // Take exactly one frame from the buffer
        let frame: Vec<f32> = self.sample_buf.drain(..self.frame_size).collect();

        // Convert f32 to i16 (webrtc-vad expects i16 PCM)
        let frame_i16: Vec<i16> = frame
            .iter()
            .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
            .collect();

        let is_voice = self.vad.is_voice_segment(&frame_i16).unwrap_or(false);

        match self.state {
            VadState::Silence => {
                if is_voice {
                    self.speech_frames += 1;
                    if self.speech_frames >= self.min_speech_frames {
                        self.state = VadState::Speaking;
                        self.silence_frames = 0;
                        debug!("WebRTC VAD: Speech started");
                        return VadEvent::SpeechStart;
                    }
                } else {
                    self.speech_frames = 0;
                }
                VadEvent::Silence
            }
            VadState::Speaking => {
                if is_voice {
                    self.silence_frames = 0;
                } else {
                    self.silence_frames += 1;
                    if self.silence_frames >= self.min_silence_frames {
                        self.state = VadState::Silence;
                        self.speech_frames = 0;
                        self.silence_frames = 0;
                        debug!("WebRTC VAD: Speech ended");
                        return VadEvent::SpeechEnd { duration_chunks: 0 };
                    }
                }
                VadEvent::Silence
            }
        }
    }

    pub fn is_speaking(&self) -> bool {
        self.state == VadState::Speaking
    }

    pub fn reset(&mut self) {
        self.state = VadState::Silence;
        self.speech_frames = 0;
        self.silence_frames = 0;
        self.sample_buf.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rms_silence() {
        let silence = vec![0.0f32; 16000];
        let rms = calculate_rms(&silence);
        assert!(rms < 0.001);
    }

    #[test]
    fn test_rms_loud() {
        let loud = vec![0.5f32; 16000];
        let rms = calculate_rms(&loud);
        assert!((rms - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_vad_silence() {
        let mut vad = VadDetector::default_config();
        let silence = vec![0.001f32; 16000];
        let event = vad.process(&silence);
        assert!(matches!(event, VadEvent::Silence));
    }

    #[test]
    fn test_vad_speech_detection() {
        let mut vad = VadDetector::default_config();
        let loud = vec![0.3f32; 16000]; // loud signal

        // First chunk: not enough (min_speech_chunks=2)
        let event = vad.process(&loud);
        assert!(matches!(event, VadEvent::Silence));

        // Second chunk: speech starts
        let event = vad.process(&loud);
        assert!(matches!(event, VadEvent::SpeechStart));
        assert!(vad.is_speaking());
    }

    #[test]
    fn test_webrtc_vad_reset() {
        let mut vad = WebRtcVadDetector::default_config();
        assert!(!vad.is_speaking());
        vad.reset();
        assert!(!vad.is_speaking());
    }
}
