use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    #[must_use]
    pub fn from_storage(id: String) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommandId(String);

impl CommandId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    #[must_use]
    pub fn from_storage(id: String) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for CommandId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewSession {
    pub id: SessionId,
    pub os_context: String,
    pub hostname: String,
    pub shell: Option<String>,
    pub started_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: SessionId,
    pub os_context: String,
    pub hostname: String,
    pub shell: Option<String>,
    pub started_at_ms: i64,
    pub ended_at_ms: Option<i64>,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewCommand {
    pub id: CommandId,
    pub session_id: SessionId,
    pub command: String,
    pub cwd: String,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<i64>,
    pub started_at_ms: Option<i64>,
    pub completed_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandRecord {
    pub id: CommandId,
    pub session_id: SessionId,
    pub command: String,
    pub normalized_command: String,
    pub cwd: String,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<i64>,
    pub started_at_ms: Option<i64>,
    pub completed_at_ms: i64,
    pub content_hash: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandMetadata {
    pub command_id: CommandId,
    pub run_count: u64,
    pub first_seen_at_ms: i64,
    pub last_seen_at_ms: i64,
    pub last_exit_code: Option<i32>,
    pub last_duration_ms: Option<i64>,
    pub embedding_status: EmbeddingStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmbeddingStatus {
    Pending,
    Indexed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuedCommand {
    pub id: i64,
    pub command_id: CommandId,
    pub queue_kind: QueueKind,
    pub status: QueueStatus,
    pub attempts: u32,
    pub available_at_ms: i64,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueKind {
    Embedding,
    FtsRebuild,
    MetadataRepair,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueStatus {
    Pending,
    Processing,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturePayload {
    pub session_id: SessionId,
    pub command: String,
    pub cwd: String,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<i64>,
    pub started_at_ms: Option<i64>,
    pub completed_at_ms: i64,
}

impl CapturePayload {
    pub fn encode(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}

// ─── Search types ────────────────────────────────────────────────────────────

/// A search query with options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub limit: u32,
}

/// Configurable scoring weights for search ranking.
/// All weights should sum to 1.0 for normalized scoring.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ScoringWeights {
    /// Weight for match quality (exact > prefix > partial > fuzzy).
    pub exact_weight: f64,
    /// Weight for recency (newer = higher score).
    pub recency_weight: f64,
    /// Weight for frequency (more runs = higher score).
    pub frequency_weight: f64,
    /// Weight for cwd similarity (same directory = higher score).
    pub cwd_weight: f64,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            exact_weight: 0.50,
            recency_weight: 0.20,
            frequency_weight: 0.20,
            cwd_weight: 0.10,
        }
    }
}

impl PartialEq for ScoringWeights {
    fn eq(&self, other: &Self) -> bool {
        self.exact_weight == other.exact_weight
            && self.recency_weight == other.recency_weight
            && self.frequency_weight == other.frequency_weight
            && self.cwd_weight == other.cwd_weight
    }
}

impl Eq for ScoringWeights {}

/// Search options controlling filtering and ranking behavior.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchOptions {
    pub query: String,
    pub limit: u32,
    /// Optional cwd to boost results from the same directory.
    pub cwd: Option<String>,
    /// If true, sort purely by recency (ignoring scoring weights).
    pub recent_only: bool,
    /// Scoring weights for the ranking algorithm.
    pub weights: ScoringWeights,
}

impl SearchOptions {
    /// Create search options with defaults for the given query.
    #[must_use]
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            limit: 20,
            cwd: None,
            recent_only: false,
            weights: ScoringWeights::default(),
        }
    }

    #[must_use]
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }

    #[must_use]
    pub fn with_cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    #[must_use]
    pub fn with_recent_only(mut self, recent_only: bool) -> Self {
        self.recent_only = recent_only;
        self
    }
}

/// The kind of match that produced a search result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchKind {
    /// The query string appears verbatim in the command text.
    Exact,
    /// The query matched as a prefix of a token in the command.
    Prefix,
    /// The query matched via FTS5 trigram index (partial/substring match).
    Partial,
    /// The query matched via edit-distance fuzzy matching.
    Fuzzy,
}

/// A single search result with ranking metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResult {
    pub command: String,
    pub cwd: String,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<i64>,
    pub completed_at_ms: i64,
    pub run_count: u64,
    pub match_kind: MatchKind,
    /// Composite score in [0.0, 1.0] based on weighted ranking.
    pub score: f64,
}
