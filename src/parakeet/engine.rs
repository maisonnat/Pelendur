use std::path::{Path, PathBuf};

use crate::parakeet::error::{ParakeetError, Result};

const HF_REPO: &str = "istupakov/parakeet-tdt-0.6b-v3-onnx";
const HF_BASE_URL: &str = "https://huggingface.co";

/// Engine managing Parakeet model downloads and file lifecycle.
pub struct ParakeetEngine {
    models_dir: PathBuf,
}

impl ParakeetEngine {
    /// Create a new engine, ensuring the models directory exists.
    pub fn new(models_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&models_dir).map_err(|e| {
            ParakeetError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create model directory {:?}: {}", models_dir, e),
            ))
        })?;
        Ok(Self { models_dir })
    }

    /// Returns the path to the models directory.
    pub fn model_dir(&self) -> &Path {
        &self.models_dir
    }

    /// Check if all required model files are present with non-zero size.
    /// For files with int8/FP32 variants, checks if at least one variant exists.
    pub fn is_model_ready(&self) -> bool {
        let has_encoder = self.has_nonzero_file("encoder-model.int8.onnx")
            || self.has_nonzero_file("encoder-model.onnx");
        let has_decoder = self.has_nonzero_file("decoder_joint-model.int8.onnx")
            || self.has_nonzero_file("decoder_joint-model.onnx");
        let has_preprocessor = self.has_nonzero_file("nemo128.onnx");
        let has_vocab = self.has_nonzero_file("vocab.txt");

        has_encoder && has_decoder && has_preprocessor && has_vocab
    }

    /// Download all missing model files. Skips files that already exist with non-zero size.
    pub async fn ensure_models(&self) -> Result<()> {
        if self
            .resolve_variant("encoder-model.int8.onnx", "encoder-model.onnx")
            .is_none()
        {
            self.download_file("encoder-model.int8.onnx").await?;
        }
        if self
            .resolve_variant("decoder_joint-model.int8.onnx", "decoder_joint-model.onnx")
            .is_none()
        {
            self.download_file("decoder_joint-model.int8.onnx")
                .await?;
        }
        if !self.has_nonzero_file("nemo128.onnx") {
            self.download_file("nemo128.onnx").await?;
        }
        if !self.has_nonzero_file("vocab.txt") {
            self.download_file("vocab.txt").await?;
        }
        Ok(())
    }

    /// Get the path to use for the encoder model (int8 if available, FP32 fallback).
    pub fn encoder_path(&self) -> Option<PathBuf> {
        self.resolve_variant("encoder-model.int8.onnx", "encoder-model.onnx")
            .map(|name| self.models_dir.join(name))
    }

    /// Get the path to use for the decoder_joint model.
    pub fn decoder_path(&self) -> Option<PathBuf> {
        self.resolve_variant("decoder_joint-model.int8.onnx", "decoder_joint-model.onnx")
            .map(|name| self.models_dir.join(name))
    }

    /// Get the path to the preprocessor model.
    pub fn preprocessor_path(&self) -> PathBuf {
        self.models_dir.join("nemo128.onnx")
    }

    /// Get the path to the vocabulary file.
    pub fn vocab_path(&self) -> PathBuf {
        self.models_dir.join("vocab.txt")
    }

    fn has_nonzero_file(&self, name: &str) -> bool {
        let path = self.models_dir.join(name);
        path.exists() && path.metadata().map(|m| m.len() > 0).unwrap_or(false)
    }

    /// Resolve which file to use for variant pairs (prefers int8).
    fn resolve_variant(&self, int8_name: &str, fp32_name: &str) -> Option<String> {
        if self.has_nonzero_file(int8_name) {
            Some(int8_name.to_string())
        } else if self.has_nonzero_file(fp32_name) {
            Some(fp32_name.to_string())
        } else {
            None
        }
    }

    async fn download_file(&self, filename: &str) -> Result<()> {
        if self.has_nonzero_file(filename) {
            tracing::info!("Model file {} already exists, skipping download", filename);
            return Ok(());
        }

        let url = format!("{}/{}/resolve/main/{}", HF_BASE_URL, HF_REPO, filename);
        tracing::info!("Downloading model file: {} from {}", filename, url);

        let response = reqwest::get(&url).await.map_err(|e| {
            ParakeetError::DownloadFailed(format!("Failed to download {}: {}", filename, e))
        })?;

        if !response.status().is_success() {
            return Err(ParakeetError::DownloadFailed(format!(
                "HTTP {} downloading {}",
                response.status(),
                filename
            )));
        }

        let total_size = response.content_length();
        let dest_path = self.models_dir.join(filename);

        let temp_path = dest_path.with_extension("tmp");
        let mut file =
            tokio::fs::File::create(&temp_path)
                .await
                .map_err(ParakeetError::Io)?;

        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();
        let mut last_logged_mb: u64 = 0;

        use futures::StreamExt;
        use tokio::io::AsyncWriteExt;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| {
                ParakeetError::DownloadFailed(format!(
                    "Error reading chunk for {}: {}",
                    filename, e
                ))
            })?;
            file.write_all(&chunk).await.map_err(ParakeetError::Io)?;
            downloaded += chunk.len() as u64;

            let current_mb = downloaded / (50 * 1024 * 1024);
            if current_mb > last_logged_mb {
                last_logged_mb = current_mb;
                if let Some(total) = total_size {
                    tracing::info!(
                        "Downloading {}: {} / {} ({:.1}%)",
                        filename,
                        bytes_to_human(downloaded),
                        bytes_to_human(total),
                        (downloaded as f64 / total as f64) * 100.0
                    );
                } else {
                    tracing::info!(
                        "Downloading {}: {}",
                        filename,
                        bytes_to_human(downloaded)
                    );
                }
            }
        }

        file.flush().await.map_err(ParakeetError::Io)?;
        drop(file);

        std::fs::rename(&temp_path, &dest_path).map_err(|e| {
            ParakeetError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to rename temp file for {}: {}", filename, e),
            ))
        })?;

        tracing::info!("Downloaded {} ({})", filename, bytes_to_human(downloaded));
        Ok(())
    }
}

fn bytes_to_human(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_new_creates_directory() {
        let dir = tempfile::tempdir().unwrap();
        let model_dir = dir.path().join("models").join("parakeet");
        assert!(!model_dir.exists());
        let engine = ParakeetEngine::new(model_dir.clone()).unwrap();
        assert!(model_dir.exists());
        assert_eq!(engine.model_dir(), model_dir);
    }

    #[test]
    fn test_is_model_ready_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let model_dir = dir.path().join("parakeet");
        let engine = ParakeetEngine::new(model_dir).unwrap();
        assert!(!engine.is_model_ready());
    }

    #[test]
    fn test_is_model_ready_with_int8_files() {
        let dir = tempfile::tempdir().unwrap();
        let model_dir = dir.path().join("parakeet");
        let engine = ParakeetEngine::new(model_dir).unwrap();

        let files = [
            "encoder-model.int8.onnx",
            "decoder_joint-model.int8.onnx",
            "nemo128.onnx",
            "vocab.txt",
        ];
        for name in &files {
            fs::write(engine.model_dir().join(name), "test data").unwrap();
        }
        assert!(engine.is_model_ready());
    }

    #[test]
    fn test_is_model_ready_fp32_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let model_dir = dir.path().join("parakeet");
        let engine = ParakeetEngine::new(model_dir).unwrap();

        let files = [
            "encoder-model.onnx",
            "decoder_joint-model.onnx",
            "nemo128.onnx",
            "vocab.txt",
        ];
        for name in &files {
            fs::write(engine.model_dir().join(name), "test data").unwrap();
        }
        assert!(engine.is_model_ready());
    }

    #[test]
    fn test_is_model_ready_zero_size_file() {
        let dir = tempfile::tempdir().unwrap();
        let model_dir = dir.path().join("parakeet");
        let engine = ParakeetEngine::new(model_dir).unwrap();

        let files = [
            "encoder-model.int8.onnx",
            "decoder_joint-model.int8.onnx",
            "nemo128.onnx",
            "vocab.txt",
        ];
        for (i, name) in files.iter().enumerate() {
            if i == 0 {
                fs::File::create(engine.model_dir().join(name)).unwrap(); // zero-size
            } else {
                fs::write(engine.model_dir().join(name), "test data").unwrap();
            }
        }
        assert!(!engine.is_model_ready());
    }

    #[test]
    fn test_encoder_path_prefers_int8() {
        let dir = tempfile::tempdir().unwrap();
        let model_dir = dir.path().join("parakeet");
        let engine = ParakeetEngine::new(model_dir).unwrap();

        fs::write(engine.model_dir().join("encoder-model.int8.onnx"), "int8").unwrap();
        fs::write(engine.model_dir().join("encoder-model.onnx"), "fp32").unwrap();

        let path = engine.encoder_path().unwrap();
        assert!(path.to_str().unwrap().contains("int8"));
    }

    #[test]
    fn test_bytes_to_human() {
        assert_eq!(bytes_to_human(500), "500 B");
        assert_eq!(bytes_to_human(1024), "1.0 KB");
        assert_eq!(bytes_to_human(1024 * 1024), "1.0 MB");
        assert_eq!(bytes_to_human(150 * 1024 * 1024), "150.0 MB");
    }
}
