//! Real neural embedding provider using ONNX Runtime + all-MiniLM-L6-v2.
//!
//! This module is only compiled when the `onnx` feature is enabled.
//! It provides `MiniLmEmbeddingProvider` which performs actual transformer
//! inference: tokenize → ONNX forward pass → mean pooling → L2 normalize.
//!
//! The result is a 384-dimensional sentence embedding suitable for
//! cosine-similarity semantic search.
//!
//! All FFI interaction is encapsulated inside the `ort` and `tokenizers`
//! dependency crates — this module contains zero `unsafe` code.

use std::path::Path;
use std::sync::{Arc, Mutex};

use ort::session::Session;
use ort::value::Tensor;
use tokenizers::Tokenizer;

use crate::embedding::EmbeddingProvider;
use crate::error::{AiError, AiResult};
use crate::vector::EMBEDDING_DIMENSIONS;

/// Neural embedding provider backed by the all-MiniLM-L6-v2 ONNX model.
///
/// Implements the `EmbeddingProvider` trait for real semantic inference.
/// The model and tokenizer are loaded once and shared via `Arc`, making
/// cloning cheap for caching across daemon requests.
///
/// The `Mutex` around `Session` is required because `ort` 2.x's
/// `Session::run()` takes `&mut self`. The mutex is uncontended in
/// single-threaded CLI usage and briefly held during inference.
pub struct MiniLmEmbeddingProvider {
    session: Arc<Mutex<Session>>,
    tokenizer: Arc<Tokenizer>,
    model_name: String,
}

// Manual Clone because Arc<Mutex<Session>>/Arc<Tokenizer> are cheap to clone.
impl Clone for MiniLmEmbeddingProvider {
    fn clone(&self) -> Self {
        Self {
            session: Arc::clone(&self.session),
            tokenizer: Arc::clone(&self.tokenizer),
            model_name: self.model_name.clone(),
        }
    }
}

impl MiniLmEmbeddingProvider {
    /// Load the ONNX model and tokenizer from a model directory.
    ///
    /// Expects:
    /// - `model_dir/model.onnx` — the ONNX model weights
    /// - `model_dir/tokenizer.json` — the HuggingFace tokenizer config
    ///
    /// Returns an error if either file is missing or cannot be loaded.
    pub fn load(model_dir: &Path) -> AiResult<Self> {
        Self::load_with_name(model_dir, None)
    }

    /// Load the ONNX model and tokenizer from a directory with an explicit model name.
    ///
    /// If `name` is `None`, the directory basename is used as the model name.
    pub fn load_with_name(model_dir: &Path, name: Option<&str>) -> AiResult<Self> {
        let model_name = name
            .map(|n| n.to_owned())
            .unwrap_or_else(|| {
                model_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_owned()
            });
        let model_path = model_dir.join("model.onnx");
        let tokenizer_path = model_dir.join("tokenizer.json");

        if !model_path.exists() {
            return Err(AiError::ModelNotInstalled(format!(
                "model.onnx not found in {}",
                model_dir.display()
            )));
        }
        if !tokenizer_path.exists() {
            return Err(AiError::ModelNotInstalled(format!(
                "tokenizer.json not found in {}",
                model_dir.display()
            )));
        }

        // Load ONNX session with CPU-only, single-threaded inference.
        let session = Session::builder()
            .map_err(|e| AiError::OnnxError(format!("session builder: {e}")))?
            .with_intra_threads(1)
            .map_err(|e| AiError::OnnxError(format!("set threads: {e}")))?
            .commit_from_file(&model_path)
            .map_err(|e| AiError::OnnxError(format!("load model: {e}")))?;

        // Load HuggingFace tokenizer.
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| AiError::TokenizerError(format!("load tokenizer: {e}")))?;

        Ok(Self {
            session: Arc::new(Mutex::new(session)),
            tokenizer: Arc::new(tokenizer),
            model_name,
        })
    }

    /// Generate a 384-dimensional embedding from text.
    ///
    /// Pipeline:
    /// 1. Tokenize text → `input_ids` + `attention_mask`
    /// 2. Run ONNX inference → `last_hidden_state` [1, seq_len, 384]
    /// 3. Mean pooling (masked) → [384]
    /// 4. L2 normalize to unit vector
    fn embed_text(&self, text: &str) -> AiResult<Vec<f32>> {
        // 1. Tokenize.
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| AiError::TokenizerError(format!("encode: {e}")))?;

        let input_ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        let attention_mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|&m| m as i64)
            .collect();
        let token_type_ids: Vec<i64> = encoding.get_type_ids().iter().map(|&t| t as i64).collect();

        let seq_len = input_ids.len();

        // Build ort Tensor values from flat vectors + shapes.
        let ids_tensor = Tensor::from_array(([1usize, seq_len], input_ids))
            .map_err(|e| AiError::OnnxError(format!("create input_ids tensor: {e}")))?;
        let mask_tensor = Tensor::from_array(([1usize, seq_len], attention_mask.clone()))
            .map_err(|e| AiError::OnnxError(format!("create attention_mask tensor: {e}")))?;
        let type_tensor = Tensor::from_array(([1usize, seq_len], token_type_ids))
            .map_err(|e| AiError::OnnxError(format!("create token_type_ids tensor: {e}")))?;

        // 2. Run ONNX inference (needs &mut self on Session).
        let mut session = self
            .session
            .lock()
            .map_err(|e| AiError::OnnxError(format!("session lock poisoned: {e}")))?;

        let outputs = session
            .run(ort::inputs![ids_tensor, mask_tensor, type_tensor])
            .map_err(|e| AiError::OnnxError(format!("run inference: {e}")))?;

        // 3. Extract last_hidden_state [1, seq_len, hidden_size].
        let (shape, data) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| AiError::OnnxError(format!("extract output: {e}")))?;

        if shape.len() != 3 {
            return Err(AiError::EmbeddingFailed(format!(
                "unexpected output shape: {shape:?}, expected [1, seq_len, hidden_size]"
            )));
        }
        let hidden_size = shape[2] as usize;

        // 4. Mean pooling with attention mask.
        // data is a flat [1 * seq_len * hidden_size] slice.
        let mut pooled = vec![0.0f32; hidden_size];
        let mut mask_sum = 0.0f32;

        for (t, &mask_val_i64) in attention_mask.iter().enumerate().take(seq_len) {
            let mask_val = mask_val_i64 as f32;
            mask_sum += mask_val;
            let offset = t * hidden_size;
            for h in 0..hidden_size {
                pooled[h] += data[offset + h] * mask_val;
            }
        }

        if mask_sum > 0.0 {
            for v in &mut pooled {
                *v /= mask_sum;
            }
        }

        // 5. L2 normalize to unit vector.
        let magnitude: f32 = pooled.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for v in &mut pooled {
                *v /= magnitude;
            }
        }

        // Sanity check: MiniLM-L6-v2 should output 384 dimensions.
        if pooled.len() != EMBEDDING_DIMENSIONS {
            return Err(AiError::EmbeddingFailed(format!(
                "expected {} dimensions, got {}",
                EMBEDDING_DIMENSIONS,
                pooled.len()
            )));
        }

        Ok(pooled)
    }
}

impl EmbeddingProvider for MiniLmEmbeddingProvider {
    fn embed_query(&self, query: &str) -> Result<Vec<f32>, AiError> {
        self.embed_text(query)
    }

    fn embed_command(&self, command: &str) -> Result<Vec<f32>, AiError> {
        self.embed_text(command)
    }

    fn dimensions(&self) -> usize {
        EMBEDDING_DIMENSIONS
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

/// Check whether the ONNX model files exist in a directory.
///
/// Returns `true` if both `model.onnx` and `tokenizer.json` are present.
pub fn has_onnx_model(model_dir: &Path) -> bool {
    model_dir.join("model.onnx").exists() && model_dir.join("tokenizer.json").exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Get the default model directory for testing.
    /// Tests in this module require the model to be installed first.
    fn test_model_dir() -> PathBuf {
        if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
            return PathBuf::from(data_home)
                .join("ggnmem")
                .join("ai")
                .join("models")
                .join("all-MiniLM-L6-v2");
        }
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("ggnmem")
                .join("ai")
                .join("models")
                .join("all-MiniLM-L6-v2");
        }
        PathBuf::from("ggnmem-ai-data/models/all-MiniLM-L6-v2")
    }

    #[test]
    fn test_has_onnx_model_missing() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(!has_onnx_model(tmp.path()));
    }

    #[test]
    fn test_load_fails_without_model() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = MiniLmEmbeddingProvider::load(tmp.path());
        assert!(result.is_err());
    }

    #[test]
    #[ignore] // Requires model installed: ggnmem ai install
    fn test_minilm_load_and_embed() {
        let model_dir = test_model_dir();
        let provider = MiniLmEmbeddingProvider::load(&model_dir).unwrap();
        let embedding = provider.embed_query("docker compose up").unwrap();
        assert_eq!(embedding.len(), EMBEDDING_DIMENSIONS);
    }

    #[test]
    #[ignore] // Requires model installed
    fn test_minilm_deterministic() {
        let model_dir = test_model_dir();
        let provider = MiniLmEmbeddingProvider::load(&model_dir).unwrap();
        let e1 = provider.embed_query("hello world").unwrap();
        let e2 = provider.embed_query("hello world").unwrap();
        assert_eq!(e1, e2);
    }

    #[test]
    #[ignore] // Requires model installed
    fn test_minilm_normalized() {
        let model_dir = test_model_dir();
        let provider = MiniLmEmbeddingProvider::load(&model_dir).unwrap();
        let embedding = provider.embed_query("test normalization").unwrap();
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (magnitude - 1.0).abs() < 0.001,
            "expected unit vector, got magnitude {magnitude}"
        );
    }

    #[test]
    #[ignore] // Requires model installed
    fn test_minilm_semantic_similarity() {
        let model_dir = test_model_dir();
        let provider = MiniLmEmbeddingProvider::load(&model_dir).unwrap();

        let e_git_status = provider.embed_command("git status").unwrap();
        let e_check_git = provider.embed_query("check git changes").unwrap();
        let e_npm_install = provider.embed_command("npm install").unwrap();

        // "check git changes" should be closer to "git status"
        // than to "npm install".
        let sim_related = cosine_similarity(&e_check_git, &e_git_status);
        let sim_unrelated = cosine_similarity(&e_check_git, &e_npm_install);

        assert!(
            sim_related > sim_unrelated,
            "'check git changes' should be more similar to 'git status' ({sim_related:.4}) \
             than to 'npm install' ({sim_unrelated:.4})"
        );
    }

    #[cfg(test)]
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
    }
}
