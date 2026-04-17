use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParakeetError {
    #[error("ONNX Runtime error: {0}")]
    Ort(#[from] ort::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Shape error: {0}")]
    Shape(#[from] ndarray::ShapeError),
    #[error("Model input not found: {0}")]
    InputNotFound(String),
    #[error("Model output not found: {0}")]
    OutputNotFound(String),
    #[error("Tensor shape error for: {0}")]
    TensorShape(String),
    #[error("Model not loaded")]
    ModelNotLoaded,
    #[error("Model download failed: {0}")]
    DownloadFailed(String),
    #[error("Vocabulary error: {0}")]
    Vocab(String),
}

pub type Result<T> = std::result::Result<T, ParakeetError>;

impl From<ort::Error<ort::session::builder::SessionBuilder>> for ParakeetError {
    fn from(e: ort::Error<ort::session::builder::SessionBuilder>) -> Self {
        ParakeetError::Ort(e.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: ParakeetError = io_err.into();
        assert!(matches!(err, ParakeetError::Io(_)));
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn test_input_not_found_display() {
        let err = ParakeetError::InputNotFound("waveforms".to_string());
        assert!(err.to_string().contains("waveforms"));
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn test_download_failed_display() {
        let err = ParakeetError::DownloadFailed("HTTP 403".to_string());
        assert!(err.to_string().contains("403"));
    }

    #[test]
    fn test_vocab_error_display() {
        let err = ParakeetError::Vocab("Missing <blk>".to_string());
        assert!(err.to_string().contains("blk"));
    }

    #[test]
    fn test_model_not_loaded_display() {
        let err = ParakeetError::ModelNotLoaded;
        assert!(err.to_string().contains("not loaded"));
    }

    #[test]
    fn test_tensor_shape_display() {
        let err = ParakeetError::TensorShape("input_states_1".to_string());
        assert!(err.to_string().contains("input_states_1"));
    }
}
