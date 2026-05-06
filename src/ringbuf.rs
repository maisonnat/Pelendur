use std::collections::VecDeque;

/// Circular audio buffer for pre-roll (lookahead).
///
/// Continuously stores the last N ms of audio. When VAD detects
/// `SpeechStart`, `drain()` returns the buffered audio + current chunk
/// so the speech segment begins with natural pre-speech context,
/// preventing clipped first syllables.
///
/// # Design
/// - Capacity is fixed at construction time (pre_roll_ms at sample_rate).
/// - `push()` appends samples; oldest are evicted when full.
/// - `drain()` returns all buffered samples and clears the buffer.
/// - Thread-safe: intended for single-threaded use inside the capture thread.
pub struct AudioRingBuffer {
    buf: VecDeque<f32>,
    capacity: usize,
}

impl AudioRingBuffer {
    /// Create a new ring buffer that holds `pre_roll_ms` of audio at `sample_rate`.
    pub fn new(pre_roll_ms: u32, sample_rate: u32) -> Self {
        let capacity = (sample_rate as usize * pre_roll_ms as usize + 999) / 1000;
        Self {
            buf: VecDeque::with_capacity(capacity.min(48000)),  // cap at 1s @ 48kHz
            capacity: capacity.min(48000),
        }
    }

    /// Default: 200ms pre-roll at 16kHz = 3200 samples.
    pub fn default_config() -> Self {
        Self::new(200, 16000)
    }

    /// Push audio samples into the buffer. Oldest samples are dropped
    /// when capacity is exceeded.
    pub fn push(&mut self, samples: &[f32]) {
        for &s in samples {
            if self.buf.len() >= self.capacity {
                self.buf.pop_front();
            }
            self.buf.push_back(s);
        }
    }

    /// Drain all buffered samples and return them as a Vec.
    /// After this call, the buffer is empty.
    pub fn drain(&mut self) -> Vec<f32> {
        self.buf.drain(..).collect()
    }

    /// Clear the buffer without returning samples.
    pub fn clear(&mut self) {
        self.buf.clear();
    }

    /// Current number of samples stored.
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_push_and_drain() {
        let mut rb = AudioRingBuffer::new(100, 16000); // 100ms @ 16kHz = 1600 samples
        let chunk: Vec<f32> = vec![0.5; 800]; // 50ms
        rb.push(&chunk);
        assert_eq!(rb.len(), 800);
        let drained = rb.drain();
        assert_eq!(drained.len(), 800);
        assert!(rb.is_empty());
    }

    #[test]
    fn test_capacity_eviction() {
        // Buffer holds 100ms @ 16kHz = 1600 samples
        let mut rb = AudioRingBuffer::new(100, 16000);
        // Push 2000 samples (125ms) — oldest 400 should be evicted
        let chunk: Vec<f32> = vec![1.0; 2000];
        rb.push(&chunk);
        assert_eq!(rb.len(), 1600);
        // All values should be 1.0 (all from the chunk, just truncated)
        for &s in rb.buf.iter() {
            assert!((s - 1.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_eviction_oldest_dropped() {
        let mut rb = AudioRingBuffer::new(100, 16000); // 1600 samples
        // Push 1600 samples of 0.5 — fills the buffer
        let first: Vec<f32> = vec![0.5; 1600];
        rb.push(&first);
        // Push 800 samples of 1.0 — oldest 800 of 0.5 should be evicted
        let second: Vec<f32> = vec![1.0; 800];
        rb.push(&second);
        assert_eq!(rb.len(), 1600);
        let drained = rb.drain();
        // First 800 should be 0.5 (survivors from original 1600), last 800 should be 1.0
        for (i, &s) in drained.iter().enumerate() {
            if i < 800 {
                assert!((s - 0.5).abs() < 0.01, "index {} should be 0.5, got {}", i, s);
            } else {
                assert!((s - 1.0).abs() < 0.01, "index {} should be 1.0, got {}", i, s);
            }
        }
    }

    #[test]
    fn test_clear() {
        let mut rb = AudioRingBuffer::new(100, 16000);
        rb.push(&vec![0.5; 800]);
        assert!(!rb.is_empty());
        rb.clear();
        assert!(rb.is_empty());
        assert_eq!(rb.len(), 0);
    }
}
