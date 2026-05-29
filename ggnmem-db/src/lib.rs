pub mod config;
pub mod connection;
pub mod domain;
pub mod error;
pub mod filter;
pub mod fuzzy;
pub mod hash;
pub mod migrations;
pub mod storage;
pub mod time;

pub use config::{AppConfig, DatabaseConfig, ModelConfig, RuntimeConfig};
pub use connection::{open_database, open_database_at};
pub use domain::{
    CapturePayload, CleanupMode, CommandId, CommandMetadata, CommandRecord, DbStats,
    EmbeddingStatus, MatchKind, NewCommand, NewSession, OptimizeStats, QueueKind, QueueStatus,
    QueuedCommand, ScoringWeights, SearchOptions, SearchQuery, SearchResult, SessionId,
    SessionRecord, UsageStats,
};
pub use error::{DbError, DbResult};
pub use filter::{is_internal_command, should_ingest};
pub use storage::{CleanupStats, Database};
