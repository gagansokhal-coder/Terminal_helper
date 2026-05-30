//! AI-specific error types for the ggnmem-ai crate.

use thiserror::Error;

pub type AiResult<T> = Result<T, AiError>;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("AI features are not enabled")]
    NotEnabled,

    #[error("model not installed: {0}")]
    ModelNotInstalled(String),

    #[error("model already installed: {0}")]
    ModelAlreadyInstalled(String),

    #[error("unknown model: {0}")]
    UnknownModel(String),

    #[error("embedding failed: {0}")]
    EmbeddingFailed(String),

    #[error("vector database error: {0}")]
    VectorDbError(String),

    #[error("database error: {0}")]
    Db(#[from] ggnmem_db::DbError),

    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
