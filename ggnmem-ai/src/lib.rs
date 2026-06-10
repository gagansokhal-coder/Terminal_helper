//! Optional AI embedding and vector search module for ggnmem.
//!
//! This crate is responsible for:
//! - Local model management (install, download, verify)
//! - Embedding pipeline interfaces (traits + implementations)
//! - Neural ONNX inference (feature = "onnx")
//! - Vector storage layer (lazy-initialized, separate from core DB)
//! - Background indexing of historical commands
//!
//! This crate does NOT:
//! - Implement chat, agents, or code generation
//! - Download models from the internet (unless user runs `ai install`)
//! - Affect core ggnmem functionality when disabled

pub mod config;
pub mod embedding;
pub mod error;
pub mod indexer;
pub mod models;
#[cfg(feature = "onnx")]
pub mod onnx;
pub mod vector;

pub use config::AiConfig;
pub use embedding::{
    create_provider, EmbeddingPipeline, EmbeddingProvider, NgramEmbeddingProvider,
    TestEmbeddingProvider,
};
pub use error::{AiError, AiResult};
pub use indexer::IndexProgress;
pub use models::{ModelInfo, ModelManager};
#[cfg(feature = "onnx")]
pub use onnx::MiniLmEmbeddingProvider;
pub use vector::{VectorMatch, VectorStore};

/// Whether ONNX Runtime support was compiled in.
/// Downstream crates can use this to report build capabilities.
pub const ONNX_ENABLED: bool = cfg!(feature = "onnx");
