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
}
