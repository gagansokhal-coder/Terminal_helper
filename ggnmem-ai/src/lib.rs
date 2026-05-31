//! Optional AI embedding and vector search module for ggnmem.
//!
//! This crate is responsible for:
//! - Local model management (install, remove, list)
//! - Embedding pipeline interfaces (traits + test implementations)
//! - Vector storage layer (lazy-initialized, separate from core DB)
//! - Background indexing of historical commands
//!
//! This crate does NOT:
//! - Implement chat, agents, or code generation
//! - Include any ML runtime (candle, ONNX, etc.)
//! - Download models from the internet
//! - Affect core ggnmem functionality when disabled

pub mod config;
pub mod embedding;
pub mod error;
pub mod indexer;
pub mod models;
pub mod vector;

pub use config::AiConfig;
pub use embedding::{EmbeddingPipeline, EmbeddingProvider, TestEmbeddingProvider};
pub use error::{AiError, AiResult};
pub use indexer::IndexProgress;
pub use models::{ModelInfo, ModelManager};
pub use vector::{VectorMatch, VectorStore};
