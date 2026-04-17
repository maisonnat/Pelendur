//! Platform-aware audio strategy selection.
//!
//! Provides a unified interface for audio capture that automatically
//! selects the correct backend based on the target OS.

use anyhow::Result;
use std::sync::mpsc;

// Re-export AudioProcess for convenience on platforms that have it
#[cfg(feature = "wasapi_loopback")]
pub use crate::loopback::real::AudioProcess;

/// A captured audio source descriptor.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioSource {
    pub id: String,
    pub name: String,
}

/// Strategy trait for platform-specific audio capture.
pub trait AudioStrategy {
    fn name(&self) -> &str;
    fn start_system_capture(&self) -> Result<mpsc::Receiver<crate::audio::AudioChunk>>;
    fn list_sources(&self) -> Vec<AudioSource>;
}

// ---- Windows Strategy (WASAPI Process Loopback) ----

#[cfg(feature = "wasapi_loopback")]
pub struct WindowsStrategy {
    process_id: Option<u32>,
}

#[cfg(feature = "wasapi_loopback")]
impl WindowsStrategy {
    pub fn new() -> Self {
        Self { process_id: None }
    }

    pub fn with_process(mut self, pid: u32) -> Self {
        self.process_id = Some(pid);
        self
    }
}

#[cfg(feature = "wasapi_loopback")]
impl AudioStrategy for WindowsStrategy {
    fn name(&self) -> &str {
        "WASAPI Polling Loopback"
    }

    fn start_system_capture(&self) -> Result<mpsc::Receiver<crate::audio::AudioChunk>> {
        match self.process_id {
            Some(pid) => crate::loopback::real::start_loopback_capture(pid, true),
            None => Err(anyhow::anyhow!(
                "No process selected. Use with_process(pid) or call list_sources() first."
            )),
        }
    }

    fn list_sources(&self) -> Vec<AudioSource> {
        crate::loopback::real::list_audio_processes()
            .into_iter()
            .map(|p| AudioSource {
                id: p.pid.to_string(),
                name: p.name,
            })
            .collect()
    }
}

// ---- Linux Strategy (PulseAudio Monitor) ----

#[cfg(feature = "linux_audio")]
pub struct LinuxStrategy;

#[cfg(feature = "linux_audio")]
impl LinuxStrategy {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "linux_audio")]
impl AudioStrategy for LinuxStrategy {
    fn name(&self) -> &str {
        "PulseAudio Monitor"
    }

    fn start_system_capture(&self) -> Result<mpsc::Receiver<crate::audio::AudioChunk>> {
        crate::linux_audio::start_system_audio_capture()
    }

    fn list_sources(&self) -> Vec<AudioSource> {
        vec![AudioSource {
            id: "@DEFAULT_MONITOR@".to_string(),
            name: "System Audio (default output)".to_string(),
        }]
    }
}

// ---- Compile-time strategy detection ----

/// Detect and return the appropriate audio strategy for the current platform.
/// Uses compile-time cfg! to select the correct implementation.
pub fn detect_strategy() -> Result<Box<dyn AudioStrategy>> {
    #[cfg(feature = "wasapi_loopback")]
    {
        Ok(Box::new(WindowsStrategy::new()))
    }

    #[cfg(all(not(feature = "wasapi_loopback"), feature = "linux_audio"))]
    {
        Ok(Box::new(LinuxStrategy::new()))
    }

    #[cfg(all(not(feature = "wasapi_loopback"), not(feature = "linux_audio")))]
    {
        Err(anyhow::anyhow!(
            "No audio strategy available. Enable 'wasapi_loopback' (Windows) or 'linux_audio' (Linux) feature."
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_source_debug_clone() {
        let source = AudioSource {
            id: "test-id".to_string(),
            name: "Test Source".to_string(),
        };
        let cloned = source.clone();
        assert_eq!(cloned.id, "test-id");
        assert_eq!(cloned.name, "Test Source");
        let debug_str = format!("{:?}", source);
        assert!(debug_str.contains("test-id"));
        assert!(debug_str.contains("Test Source"));
    }

    #[test]
    fn test_audio_source_serialize_deserialize() {
        let source = AudioSource {
            id: "@DEFAULT_MONITOR@".to_string(),
            name: "System Audio".to_string(),
        };
        let json = serde_json::to_string(&source).expect("serialize");
        assert!(json.contains("@DEFAULT_MONITOR@"));
        let deserialized: AudioSource = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.id, source.id);
        assert_eq!(deserialized.name, source.name);
    }

    #[test]
    fn test_audio_source_default_values() {
        let source = AudioSource {
            id: String::new(),
            name: String::new(),
        };
        assert!(source.id.is_empty());
        assert!(source.name.is_empty());
    }

    // --- cfg-gated tests: no audio features ---

    #[test]
    #[cfg(all(not(feature = "wasapi_loopback"), not(feature = "linux_audio")))]
    fn test_detect_strategy_error_no_features() {
        match detect_strategy() {
            Ok(_) => panic!("should fail without audio features"),
            Err(e) => {
                let msg = e.to_string();
                assert!(msg.contains("wasapi_loopback"));
                assert!(msg.contains("linux_audio"));
                assert!(msg.contains("No audio strategy"));
            }
        }
    }

    // --- cfg-gated tests: linux_audio feature ---

    #[test]
    #[cfg(all(not(feature = "wasapi_loopback"), feature = "linux_audio"))]
    fn test_detect_strategy_linux_audio() {
        let result = detect_strategy();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name(), "PulseAudio Monitor");
    }

    #[test]
    #[cfg(feature = "linux_audio")]
    fn test_linux_strategy_list_sources() {
        let strategy = LinuxStrategy::new();
        let sources = strategy.list_sources();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].id, "@DEFAULT_MONITOR@");
        assert!(sources[0].name.contains("System Audio"));
    }

    #[test]
    #[cfg(feature = "linux_audio")]
    fn test_strategy_trait_object() {
        let strategy: Box<dyn AudioStrategy> = Box::new(LinuxStrategy::new());
        assert_eq!(strategy.name(), "PulseAudio Monitor");
        assert!(!strategy.list_sources().is_empty());
    }
}
