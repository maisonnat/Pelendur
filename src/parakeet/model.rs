use ndarray::{Array, Array1, Array2, Array3, ArrayD, ArrayViewD, IxDyn};
use ort::execution_providers::CPUExecutionProvider;
use ort::inputs;
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::TensorRef;
use std::fs;
use std::path::Path;

use crate::parakeet::error::{ParakeetError, Result};

pub type DecoderState = (Array3<f32>, Array3<f32>);

const SUBSAMPLING_FACTOR: usize = 8;
const WINDOW_SIZE: f32 = 0.01;
const MAX_TOKENS_PER_STEP: usize = 10;

#[derive(Debug, Clone)]
pub struct TimestampedResult {
    pub text: String,
    pub timestamps: Vec<f32>,
    pub tokens: Vec<String>,
}

pub struct ParakeetModel {
    encoder: Session,
    decoder_joint: Session,
    preprocessor: Session,
    vocab: Vec<String>,
    blank_idx: i32,
    vocab_size: usize,
}

impl ParakeetModel {
    /// Load all ONNX sessions and vocab from `model_dir`.
    ///
    /// When `quantized` is true, prefers `.int8.onnx` variants when present.
    pub fn new(model_dir: &Path, quantized: bool) -> Result<Self> {
        let num_cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        tracing::info!("Parakeet using {} physical cores (intra_op_threads)", num_cores);
        let encoder = Self::init_session(model_dir, "encoder-model", Some(num_cores), quantized)?;
        let decoder_joint = Self::init_session(model_dir, "decoder_joint-model", Some(num_cores), quantized)?;
        let preprocessor = Self::init_session(model_dir, "nemo128", Some(1), false)?;
        let (vocab, blank_idx) = Self::load_vocab(model_dir)?;
        let vocab_size = vocab.len();

        tracing::info!(
            "ParakeetModel ready — {} vocab tokens, blank_idx={}",
            vocab_size,
            blank_idx
        );

        Ok(Self {
            encoder,
            decoder_joint,
            preprocessor,
            vocab,
            blank_idx,
            vocab_size,
        })
    }

    // ── Session helpers ──────────────────────────────────────────────

    fn init_session(
        model_dir: &Path,
        model_name: &str,
        intra_threads: Option<usize>,
        try_quantized: bool,
    ) -> Result<Session> {
        let providers = vec![CPUExecutionProvider::default().build()];

        let model_filename = if try_quantized {
            let qname = format!("{}.int8.onnx", model_name);
            if model_dir.join(&qname).exists() {
                qname
            } else {
                format!("{}.onnx", model_name)
            }
        } else {
            format!("{}.onnx", model_name)
        };

        let model_path = model_dir.join(&model_filename);
        tracing::info!("Loading ONNX model: {:?}", model_path);

        let mut builder = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_execution_providers(providers)?
            .with_parallel_execution(true)?;

        if let Some(threads) = intra_threads {
            builder = builder
                .with_intra_threads(threads)?
                .with_inter_threads(1)?;  // inter_op=1 prevents thread contention
        }

        let session = builder.commit_from_file(&model_path)?;

        tracing::info!(
            "Loaded {} — {} inputs, {} outputs",
            model_name,
            session.inputs().len(),
            session.outputs().len()
        );
        for input in session.inputs() {
            tracing::trace!("  {}: {:?}", input.name(), input.dtype());
        }

        Ok(session)
    }

    // ── Vocab ────────────────────────────────────────────────────────

    fn load_vocab(model_dir: &Path) -> Result<(Vec<String>, i32)> {
        let content = fs::read_to_string(model_dir.join("vocab.txt"))?;
        let mut max_id = 0;
        let mut tokens_with_ids = Vec::new();
        let mut blank_idx = None;

        for line in content.lines() {
            let parts: Vec<&str> = line.trim_end().split(' ').collect();
            if parts.len() >= 2 {
                let token = parts[0].to_string();
                if let Ok(id) = parts[1].parse::<usize>() {
                    if token == "<blk>" {
                        blank_idx = Some(id);
                    }
                    tokens_with_ids.push((token, id));
                    max_id = max_id.max(id);
                }
            }
        }

        let mut vocab = vec![String::new(); max_id + 1];
        for (token, id) in tokens_with_ids {
            vocab[id] = token.replace('\u{2581}', " ");
        }

        let blank_idx =
            blank_idx.ok_or_else(|| ParakeetError::Vocab("Missing <blk> token".into()))? as i32;

        tracing::info!(
            "Loaded vocab — {} tokens, blank_idx={}",
            vocab.len(),
            blank_idx
        );
        Ok((vocab, blank_idx))
    }

    // ── Preprocess ───────────────────────────────────────────────────

    pub fn preprocess(
        &mut self,
        waveforms: &ArrayViewD<f32>,
        waveforms_lens: &ArrayViewD<i64>,
    ) -> Result<(ArrayD<f32>, ArrayD<i64>)> {
        let inputs = inputs![
            "waveforms" => TensorRef::from_array_view(waveforms.view())?,
            "waveforms_lens" => TensorRef::from_array_view(waveforms_lens.view())?,
        ];
        let outputs = self.preprocessor.run(inputs)?;

        let features = outputs
            .get("features")
            .ok_or_else(|| ParakeetError::OutputNotFound("features".into()))?
            .try_extract_array()?;
        let features_lens = outputs
            .get("features_lens")
            .ok_or_else(|| ParakeetError::OutputNotFound("features_lens".into()))?
            .try_extract_array()?;

        tracing::trace!(
            "preprocess: features shape={:?}, features_lens={:?}",
            features.shape(),
            features_lens.shape()
        );

        Ok((features.to_owned(), features_lens.to_owned()))
    }

    // ── Encode ───────────────────────────────────────────────────────

    pub fn encode(
        &mut self,
        audio_signal: &ArrayViewD<f32>,
        length: &ArrayViewD<i64>,
    ) -> Result<(ArrayD<f32>, ArrayD<i64>)> {
        let inputs = inputs![
            "audio_signal" => TensorRef::from_array_view(audio_signal.view())?,
            "length" => TensorRef::from_array_view(length.view())?,
        ];
        let outputs = self.encoder.run(inputs)?;

        let encoder_output = outputs
            .get("outputs")
            .ok_or_else(|| ParakeetError::OutputNotFound("outputs".into()))?
            .try_extract_array()?;
        let encoded_lengths = outputs
            .get("encoded_lengths")
            .ok_or_else(|| ParakeetError::OutputNotFound("encoded_lengths".into()))?
            .try_extract_array()?;

        // CRITICAL: swap last two dimensions for decoder compatibility
        let encoder_output = encoder_output.permuted_axes(IxDyn(&[0, 2, 1]));

        tracing::trace!(
            "encode: output shape={:?}, encoded_lengths={:?}",
            encoder_output.shape(),
            encoded_lengths.shape()
        );

        Ok((encoder_output.to_owned(), encoded_lengths.to_owned()))
    }

    // ── Decoder state ────────────────────────────────────────────────

    pub fn create_decoder_state(&self) -> Result<DecoderState> {
        let inputs = self.decoder_joint.inputs();

        let s1 = inputs
            .iter()
            .find(|i| i.name() == "input_states_1")
            .ok_or_else(|| ParakeetError::InputNotFound("input_states_1".into()))?;
        let s2 = inputs
            .iter()
            .find(|i| i.name() == "input_states_2")
            .ok_or_else(|| ParakeetError::InputNotFound("input_states_2".into()))?;

        let s1_shape = s1
            .dtype()
            .tensor_shape()
            .ok_or_else(|| ParakeetError::TensorShape("input_states_1".into()))?;
        let s2_shape = s2
            .dtype()
            .tensor_shape()
            .ok_or_else(|| ParakeetError::TensorShape("input_states_2".into()))?;

        // Shape dims are i64; negative values are dynamic — treat as 1 for init.
        // batch_size is fixed to 1 (dim[1]).
        let state1 = Array::zeros((dim_val(s1_shape[0]), 1usize, dim_val(s1_shape[2])));
        let state2 = Array::zeros((dim_val(s2_shape[0]), 1usize, dim_val(s2_shape[2])));

        tracing::debug!(
            "Created decoder state: s1={:?}, s2={:?}",
            state1.shape(),
            state2.shape()
        );

        Ok((state1, state2))
    }

    // ── Decode step ──────────────────────────────────────────────────

    pub fn decode_step(
        &mut self,
        prev_tokens: &[i32],
        prev_state: &DecoderState,
        encoder_out: &ArrayViewD<f32>,
    ) -> Result<(ArrayD<f32>, DecoderState)> {
        let target_token = prev_tokens.last().copied().unwrap_or(self.blank_idx);

        // Shape encoder output to [1, time_steps, 1]
        let encoder_outputs = encoder_out
            .to_owned()
            .insert_axis(ndarray::Axis(0))
            .insert_axis(ndarray::Axis(2));

        let targets = Array2::from_shape_vec((1, 1), vec![target_token])?;
        let target_length = Array1::from_vec(vec![1i32]);

        let inputs = inputs![
            "encoder_outputs" => TensorRef::from_array_view(encoder_outputs.view())?,
            "targets" => TensorRef::from_array_view(targets.view())?,
            "target_length" => TensorRef::from_array_view(target_length.view())?,
            "input_states_1" => TensorRef::from_array_view(prev_state.0.view())?,
            "input_states_2" => TensorRef::from_array_view(prev_state.1.view())?,
        ];

        let outputs = self.decoder_joint.run(inputs)?;

        let logits = outputs
            .get("outputs")
            .ok_or_else(|| ParakeetError::OutputNotFound("outputs".into()))?
            .try_extract_array()?;
        let state1 = outputs
            .get("output_states_1")
            .ok_or_else(|| ParakeetError::OutputNotFound("output_states_1".into()))?
            .try_extract_array()?;
        let state2 = outputs
            .get("output_states_2")
            .ok_or_else(|| ParakeetError::OutputNotFound("output_states_2".into()))?
            .try_extract_array()?;

        // Remove batch dimension from logits: [1, vocab+duration] → [vocab+duration]
        let logits = logits.remove_axis(ndarray::Axis(0));
        // Convert flat state arrays back to Array3
        let state1_3d = state1.to_owned().into_dimensionality::<ndarray::Ix3>()?;
        let state2_3d = state2.to_owned().into_dimensionality::<ndarray::Ix3>()?;

        Ok((logits.to_owned(), (state1_3d, state2_3d)))
    }

    // ── Decode sequence (TDT autoregressive loop) ────────────────────

    fn decode_sequence(
        &mut self,
        encodings: &ArrayViewD<f32>,
        encodings_len: usize,
    ) -> Result<(Vec<i32>, Vec<usize>)> {
        let mut prev_state = self.create_decoder_state()?;
        let mut tokens = Vec::new();
        let mut timestamps = Vec::new();
        let mut t = 0;
        let mut emitted_tokens = 0;

        while t < encodings_len {
            let encoder_step = encodings.slice(ndarray::s![t, ..]);
            let encoder_step_dyn = encoder_step.to_owned().into_dyn();
            let (probs, new_state) =
                self.decode_step(&tokens, &prev_state, &encoder_step_dyn.view())?;

            // TDT model: first vocab_size values are vocab logits, rest are duration logits
            let vocab_logits_slice = probs.as_slice().ok_or_else(|| {
                ParakeetError::Shape(ndarray::ShapeError::from_kind(
                    ndarray::ErrorKind::IncompatibleShape,
                ))
            })?;
            let vocab_logits = if probs.len() > self.vocab_size {
                &vocab_logits_slice[..self.vocab_size]
            } else {
                vocab_logits_slice
            };

            let token = vocab_logits
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(idx, _)| idx as i32)
                .unwrap_or(self.blank_idx);

            if token != self.blank_idx {
                prev_state = new_state;
                tokens.push(token);
                timestamps.push(t);
                emitted_tokens += 1;
            }

            if token == self.blank_idx || emitted_tokens == MAX_TOKENS_PER_STEP {
                t += 1;
                emitted_tokens = 0;
            }
        }

        if tokens.is_empty() {
            tracing::debug!(
                "decode_sequence: zero tokens decoded from {} timesteps",
                encodings_len
            );
        }

        Ok((tokens, timestamps))
    }

    // ── Token decoding ───────────────────────────────────────────────

    fn decode_tokens(&self, ids: Vec<i32>, timestamps: Vec<usize>) -> TimestampedResult {
        let tokens: Vec<String> = ids
            .iter()
            .filter_map(|&id| {
                let idx = id as usize;
                if idx < self.vocab.len() {
                    Some(self.vocab[idx].clone())
                } else {
                    None
                }
            })
            .collect();

        let text = match decode_space_regex() {
            Ok(re) => re
                .replace_all(&tokens.join(""), |caps: &regex::Captures| {
                    if caps.get(1).is_some() {
                        " "
                    } else {
                        ""
                    }
                })
                .to_string(),
            Err(_) => tokens.join(""),
        };

        let float_timestamps: Vec<f32> = timestamps
            .iter()
            .map(|&t| WINDOW_SIZE * SUBSAMPLING_FACTOR as f32 * t as f32)
            .collect();

        TimestampedResult {
            text,
            timestamps: float_timestamps,
            tokens,
        }
    }

    // ── Batch / top-level entry points ───────────────────────────────

    pub fn recognize_batch(
        &mut self,
        waveforms: &ArrayViewD<f32>,
        waveforms_len: &ArrayViewD<i64>,
    ) -> Result<Vec<TimestampedResult>> {
        let (features, features_lens) = self.preprocess(waveforms, waveforms_len)?;
        let (encoder_out, encoder_out_lens) =
            self.encode(&features.view(), &features_lens.view())?;

        let mut results = Vec::new();
        for (encodings, &encodings_len) in encoder_out.outer_iter().zip(encoder_out_lens.iter()) {
            let (tokens, timestamps) =
                self.decode_sequence(&encodings.view(), encodings_len as usize)?;
            results.push(self.decode_tokens(tokens, timestamps));
        }

        Ok(results)
    }

    /// Transcribe a single channel of f32 samples.
    pub fn transcribe_samples(&mut self, samples: Vec<f32>) -> Result<TimestampedResult> {
        let samples_len = samples.len();
        let waveforms = Array2::from_shape_vec((1, samples_len), samples)?.into_dyn();
        let waveforms_lens = Array1::from_vec(vec![samples_len as i64]).into_dyn();
        let results = self.recognize_batch(&waveforms.view(), &waveforms_lens.view())?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| ParakeetError::Vocab("No transcription result returned".into()))
    }
}

impl Drop for ParakeetModel {
    fn drop(&mut self) {
        tracing::info!("Unloading ParakeetModel — releasing ONNX sessions");
    }
}

/// Convert a Shape dimension (i64) to usize.
/// Negative values are dynamic dimensions — default to 1 for zero-init.
fn dim_val(d: i64) -> usize {
    if d > 0 {
        d as usize
    } else {
        1
    }
}

/// Lazily compiled regex for post-processing decoded token text.
/// Replaces `\A\s|\s\B|(\s)\b` — removes leading spaces, fixes word-piece joins.
fn decode_space_regex() -> &'static std::result::Result<regex::Regex, regex::Error> {
    static RE: std::sync::OnceLock<std::result::Result<regex::Regex, regex::Error>> =
        std::sync::OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"\A\s|\s\B|(\s)\b"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_vocab(dir: &std::path::Path) {
        use std::fmt::Write;
        let mut content = String::new();
        let tokens = [
            ("<blk>", 0),
            ("▁hello", 1),
            ("▁world", 2),
            ("▁the", 3),
            ("▁a", 4),
            ("s", 5),
            ("!", 6),
            ("▁", 7),
        ];
        for (token, id) in &tokens {
            writeln!(&mut content, "{} {}", token, id).unwrap();
        }
        std::fs::write(dir.join("vocab.txt"), content).unwrap();
    }

    #[test]
    fn test_load_vocab_parses_correctly() {
        let dir = tempfile::tempdir().unwrap();
        create_test_vocab(dir.path());

        let (vocab, blank_idx) = ParakeetModel::load_vocab(dir.path()).unwrap();

        assert_eq!(blank_idx, 0); // <blk> is id 0
        assert_eq!(vocab.len(), 8);
        // ▁hello (id 1) should be stored with ▁ replaced by space
        assert_eq!(vocab[1], " hello");
        // <blk> (id 0) stays as-is
        assert_eq!(vocab[0], "<blk>");
    }

    #[test]
    fn test_load_vocab_missing_blk_errors() {
        let dir = tempfile::tempdir().unwrap();
        // Write a vocab without <blk>
        std::fs::write(dir.path().join("vocab.txt"), "hello 0\nworld 1\n").unwrap();

        let result = ParakeetModel::load_vocab(dir.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ParakeetError::Vocab(_)));
        assert!(err.to_string().contains("blk"));
    }

    #[test]
    fn test_decode_tokens_basic() {
        let dir = tempfile::tempdir().unwrap();
        create_test_vocab(dir.path());
        let _model_dir_str = dir.path().to_string_lossy().to_string();

        // We can't construct ParakeetModel without ONNX sessions,
        // but we can test decode_tokens by building a model-like struct.
        // Instead, test the regex helper directly.
        let re = decode_space_regex().as_ref().unwrap();

        // Simulate joined tokens: " hello world" (with leading space from ▁ replacement)
        let tokens = [" hello", " world"];
        let joined = tokens.join("");
        let result = re
            .replace_all(
                &joined,
                |caps: &regex::Captures| {
                    if caps.get(1).is_some() {
                        " "
                    } else {
                        ""
                    }
                },
            )
            .to_string();
        // Leading space should be removed
        assert!(
            !result.starts_with(" "),
            "Leading space should be removed: got '{}'",
            result
        );
    }

    #[test]
    fn test_dim_val_positive() {
        assert_eq!(dim_val(5), 5);
        assert_eq!(dim_val(1), 1);
    }

    #[test]
    fn test_dim_val_negative_defaults_to_one() {
        assert_eq!(dim_val(-1), 1);
        assert_eq!(dim_val(-100), 1);
    }

    #[test]
    fn test_dim_val_zero_defaults_to_one() {
        assert_eq!(dim_val(0), 1);
    }

    // Integration tests requiring real ONNX model files — tagged #[ignore]

    #[test]
    #[ignore = "Requires ONNX model files. Run with: cargo test --features parakeet -- --ignored"]
    fn test_model_loads_and_transcribes_silence() {
        let model_dir =
            std::path::PathBuf::from(std::env::var("PARAKEET_MODEL_DIR").unwrap_or_else(|_| {
                format!(
                    "{}/.local/share/pelendur/models/parakeet",
                    std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
                )
            }));
        if !model_dir.exists() {
            eprintln!("Skipping: model dir {:?} not found", model_dir);
            return;
        }
        let mut model = ParakeetModel::new(&model_dir, true).unwrap();

        let silence = vec![0.0f32; 16000];
        let result = model.transcribe_samples(silence).unwrap();
        assert!(result.text.trim().is_empty() || result.text.trim().len() < 5);
    }

    #[test]
    #[ignore = "Requires ONNX model files. Run with: cargo test --features parakeet -- --ignored"]
    fn test_model_loads_and_transcribes_sine_wave() {
        let model_dir =
            std::path::PathBuf::from(std::env::var("PARAKEET_MODEL_DIR").unwrap_or_else(|_| {
                format!(
                    "{}/.local/share/pelendur/models/parakeet",
                    std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
                )
            }));
        if !model_dir.exists() {
            eprintln!("Skipping: model dir {:?} not found", model_dir);
            return;
        }
        let mut model = ParakeetModel::new(&model_dir, true).unwrap();

        let samples: Vec<f32> = (0..32000)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 16000.0).sin() * 0.5)
            .collect();
        let result = model.transcribe_samples(samples).unwrap();
        println!("Sine wave transcription: '{}'", result.text);
    }

    #[test]
    #[ignore = "Requires ONNX model files. Run with: cargo test --features parakeet -- --ignored"]
    fn test_vocab_loading_with_real_model() {
        let model_dir =
            std::path::PathBuf::from(std::env::var("PARAKEET_MODEL_DIR").unwrap_or_else(|_| {
                format!(
                    "{}/.local/share/pelendur/models/parakeet",
                    std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
                )
            }));
        if !model_dir.exists() {
            eprintln!("Skipping: model dir {:?} not found", model_dir);
            return;
        }
        let model = ParakeetModel::new(&model_dir, true).unwrap();
        assert!(
            model.vocab_size > 100,
            "Vocab should have more than 100 tokens"
        );
    }
}
