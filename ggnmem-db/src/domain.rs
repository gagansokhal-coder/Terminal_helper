//! Domain types shared by the database layer and its callers.
//!
//! Contains the id newtypes, the insert/read models for sessions and commands, the
//! queue and metadata models, search options and results, and the statistics types.
//! All `*_ms` fields are milliseconds since the Unix epoch.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// Identifier of a shell session (a row in `sessions`).
///
/// A thin wrapper around a string, normally a random UUID v4. Ids read back from the
/// database are rebuilt with `from_storage`.
pub struct SessionId(String);

impl SessionId {
    /// Generate a fresh random id (UUID v4).
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Rebuild an id from the string stored in the database.
    ///
    /// The value is taken as is; it is not checked to be a valid UUID.
    #[must_use]
    pub fn from_storage(id: String) -> Self {
        Self(id)
    }

    /// The id as a string slice, as stored in the database.
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
/// Identifier of a stored command (a row in `commands`).
///
/// A thin wrapper around a string, normally a random UUID v4. Ids read back from the
/// database are rebuilt with `from_storage`.
pub struct CommandId(String);

impl CommandId {
    /// Generate a fresh random id (UUID v4).
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Rebuild an id from the string stored in the database.
    ///
    /// The value is taken as is; it is not checked to be a valid UUID.
    #[must_use]
    pub fn from_storage(id: String) -> Self {
        Self(id)
    }

    /// The id as a string slice, as stored in the database.
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
/// Input for [`Database::insert_session`](crate::storage::Database::insert_session): a shell
/// session that has just started.
pub struct NewSession {
    /// Unique id of the session.
    pub id: SessionId,
    /// Free-form label of the operating system the session runs on (for example `"linux"`).
    pub os_context: String,
    /// Host name of the machine the session runs on.
    pub hostname: String,
    /// Name of the shell, if known (for example `"zsh"`).
    pub shell: Option<String>,
    /// When the session started.
    pub started_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// A session as stored in the `sessions` table.
pub struct SessionRecord {
    /// Unique id of the session.
    pub id: SessionId,
    /// Free-form label of the operating system the session runs on.
    pub os_context: String,
    /// Host name of the machine the session runs on.
    pub hostname: String,
    /// Name of the shell, if known.
    pub shell: Option<String>,
    /// When the session started.
    pub started_at_ms: i64,
    /// When the session ended, or `None` if no end was recorded.
    pub ended_at_ms: Option<i64>,
    /// When the row was created. `Database::insert_session` sets this to `started_at_ms`.
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Input for [`Database::insert_command`](crate::storage::Database::insert_command): one
/// executed command, before the database derives the normalized text and content hash.
pub struct NewCommand {
    /// Id for the new row. Ignored in favour of the existing row's id when the same
    /// command in the same directory was already stored.
    pub id: CommandId,
    /// Session the command ran in. Must refer to an existing session.
    pub session_id: SessionId,
    /// The command line exactly as typed.
    pub command: String,
    /// Working directory the command ran in.
    pub cwd: String,
    /// Exit status, if it is known.
    pub exit_code: Option<i32>,
    /// How long the command ran in milliseconds, if known.
    pub duration_ms: Option<i64>,
    /// When the command started, if known.
    pub started_at_ms: Option<i64>,
    /// When the command finished. Used for ordering and recency.
    pub completed_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// A command as stored in the `commands` table.
///
/// There is one row per distinct (normalized command, cwd) pair; repeated runs update the
/// same row and bump `run_count` in [`CommandMetadata`].
pub struct CommandRecord {
    /// Unique id of the command row.
    pub id: CommandId,
    /// Session the command was first recorded in.
    pub session_id: SessionId,
    /// The command line as it was first recorded.
    pub command: String,
    /// The command with runs of whitespace collapsed to single spaces.
    pub normalized_command: String,
    /// Working directory the command ran in.
    pub cwd: String,
    /// Exit status of the latest run, if known.
    pub exit_code: Option<i32>,
    /// Duration of the latest run in milliseconds, if known.
    pub duration_ms: Option<i64>,
    /// Start time of the latest run, if known.
    pub started_at_ms: Option<i64>,
    /// Finish time of the latest run.
    pub completed_at_ms: i64,
    /// Hex SHA-256 of the normalized command and the trimmed cwd. Unique per row.
    pub content_hash: String,
    /// When the row was first created (the first run's `completed_at_ms`).
    pub created_at_ms: i64,
    /// When the row was last updated (the latest run's `completed_at_ms`).
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Per-command usage counters, stored in `command_metadata` (one row per command).
pub struct CommandMetadata {
    /// The command these counters belong to.
    pub command_id: CommandId,
    /// How many times the command has been recorded. Starts at 1.
    pub run_count: u64,
    /// Completion time of the first recorded run.
    pub first_seen_at_ms: i64,
    /// Completion time of the latest recorded run.
    pub last_seen_at_ms: i64,
    /// Exit status of the latest run, if known.
    pub last_exit_code: Option<i32>,
    /// Duration of the latest run in milliseconds, if known.
    pub last_duration_ms: Option<i64>,
    /// Whether the command has been embedded for semantic search.
    pub embedding_status: EmbeddingStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Progress of embedding a command for semantic search (`command_metadata.embedding_status`).
pub enum EmbeddingStatus {
    /// Not embedded yet. This is the default for a new command.
    Pending,
    /// Embedded and present in the vector index.
    Indexed,
    /// Embedding was attempted and failed.
    Failed,
    /// Deliberately not embedded.
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// A row of the `command_queue` table: background work waiting to be done for a command.
pub struct QueuedCommand {
    /// Auto-incremented id of the queue entry.
    pub id: i64,
    /// The command the work is for.
    pub command_id: CommandId,
    /// What kind of work this entry stands for.
    pub queue_kind: QueueKind,
    /// Where the entry is in its life cycle.
    pub status: QueueStatus,
    /// How many times processing has been attempted.
    pub attempts: u32,
    /// Earliest time the entry may be picked up.
    pub available_at_ms: i64,
    /// When the entry was created.
    pub created_at_ms: i64,
    /// When the entry was last changed.
    pub updated_at_ms: i64,
    /// Error message from the latest failed attempt, if any.
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Kind of background work in `command_queue.queue_kind`.
pub enum QueueKind {
    /// Compute the command's embedding for semantic search. The default kind.
    Embedding,
    /// Rebuild the command's full-text search entry.
    FtsRebuild,
    /// Repair the command's `command_metadata` row.
    MetadataRepair,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Life-cycle state of a [`QueuedCommand`] (`command_queue.status`).
pub enum QueueStatus {
    /// Waiting to be picked up. The default for a new entry.
    Pending,
    /// Currently being worked on.
    Processing,
    /// The last attempt failed.
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// A captured command in a compact, serializable form for passing between processes.
///
/// Like [`NewCommand`] but without an id and with a session id only; use [`Self::encode`]
/// and [`Self::decode`] to convert to and from [`bincode`] bytes.
pub struct CapturePayload {
    /// Session the command ran in.
    pub session_id: SessionId,
    /// The command line exactly as typed.
    pub command: String,
    /// Working directory the command ran in.
    pub cwd: String,
    /// Exit status, if known.
    pub exit_code: Option<i32>,
    /// How long the command ran in milliseconds, if known.
    pub duration_ms: Option<i64>,
    /// When the command started, if known.
    pub started_at_ms: Option<i64>,
    /// When the command finished.
    pub completed_at_ms: i64,
}

impl CapturePayload {
    /// Serialize the payload to [`bincode`] bytes.
    ///
    /// # Errors
    ///
    /// Returns the [`bincode::Error`] if serialization fails.
    pub fn encode(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    /// Deserialize a payload from bytes produced by [`Self::encode`].
    ///
    /// # Errors
    ///
    /// Returns the [`bincode::Error`] if `bytes` is truncated or not a valid encoded payload.
    pub fn decode(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}

// ─── Search types ────────────────────────────────────────────────────────────

/// A plain search request: query text and a result limit.
///
/// For cwd boosting, recency-only ordering or custom weights use [`SearchOptions`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchQuery {
    /// The text to search for.
    pub query: String,
    /// Maximum number of results to return.
    pub limit: u32,
}

/// Configurable scoring weights for search ranking.
/// All weights should sum to 1.0 for normalized scoring.
///
/// The defaults are 0.50 match quality, 0.20 recency, 0.20 frequency and 0.10 cwd.
/// Nothing enforces that the weights sum to 1.0.
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
    /// The text to search for.
    pub query: String,
    /// Maximum number of results to return.
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
    ///
    /// Defaults: `limit` 20, no `cwd`, `recent_only` off and [`ScoringWeights::default`].
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

    /// Set the maximum number of results and return the updated options.
    #[must_use]
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }

    /// Set the directory used to boost results from the same place and return the updated options.
    #[must_use]
    pub fn with_cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Turn recency-only ordering on or off and return the updated options.
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
    /// The command line that matched.
    pub command: String,
    /// Working directory the command ran in.
    pub cwd: String,
    /// Exit status of the latest run, if known.
    pub exit_code: Option<i32>,
    /// Duration of the latest run in milliseconds, if known.
    pub duration_ms: Option<i64>,
    /// Finish time of the latest run.
    pub completed_at_ms: i64,
    /// How many times the command was recorded (1 if no metadata row exists).
    pub run_count: u64,
    /// How the query matched the command.
    pub match_kind: MatchKind,
    /// Composite score in [0.0, 1.0] based on weighted ranking.
    pub score: f64,
}

// ─── Database diagnostics ────────────────────────────────────────────────────

/// Low-level database statistics from SQLite PRAGMAs and row counts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbStats {
    /// Rows in `commands`.
    pub command_count: u64,
    /// Rows in `sessions`.
    pub session_count: u64,
    /// Rows in `command_metadata`.
    pub metadata_count: u64,
    /// Rows in `command_queue`.
    pub queue_count: u64,
    /// Database size in bytes, computed as `page_size * page_count`.
    pub db_size_bytes: u64,
    /// SQLite page size in bytes.
    pub page_size: u64,
    /// Number of pages in the database file.
    pub page_count: u64,
    /// Number of unused pages on the SQLite freelist.
    pub freelist_count: u64,
    /// Rows in the FTS5 table, or 0 if it could not be counted.
    pub fts_row_count: u64,
    /// Sum of `run_count - 1` over commands that ran more than once.
    pub duplicate_count_estimate: u64,
    /// When `optimize` last ran, or 0 if it never did.
    pub last_optimize_at_ms: i64,
}

impl DbStats {
    /// Percentage of pages on the freelist (fragmentation estimate).
    #[must_use]
    pub fn fragmentation_pct(&self) -> f64 {
        if self.page_count == 0 {
            0.0
        } else {
            (self.freelist_count as f64 / self.page_count as f64) * 100.0
        }
    }

    /// Estimated size of the FTS index (rough: fts_row_count * 200 bytes avg).
    #[must_use]
    pub fn fts_size_estimate(&self) -> u64 {
        self.fts_row_count * 200
    }
}

/// High-level usage statistics for the `ggnmem stats` command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageStats {
    /// Rows in `commands`.
    pub total_commands: u64,
    /// Count of distinct `normalized_command` values.
    pub unique_commands: u64,
    /// Rows in `sessions`.
    pub total_sessions: u64,
    /// Total searches recorded with `record_search_performed`.
    pub searches_performed: u64,
    /// Commands with run_count > 1 (deduplicated instead of creating new rows).
    pub deduplicated_commands: u64,
    /// Top commands by run_count: `(command_text, run_count)`.
    pub most_used: Vec<(String, u64)>,
    /// Database size in bytes, computed as `page_size * page_count`.
    pub db_size_bytes: u64,
    /// When a cleanup last ran, or 0 if none was recorded.
    pub last_cleanup_at_ms: i64,
    /// Number of commands the last recorded cleanup removed.
    pub last_cleanup_removed: u64,
    /// Number of commands left after the last recorded cleanup.
    pub last_cleanup_remaining: u64,
    /// When `optimize` last ran, or 0 if it never did.
    pub last_optimize_at_ms: i64,
    /// Number of searches that used hybrid (FTS + semantic) mode.
    pub hybrid_searches: u64,
    /// Number of standalone semantic searches (`ggnmem semantic`).
    pub semantic_searches: u64,
    /// Average search latency in milliseconds (running average).
    pub avg_search_latency_ms: u64,
}

/// Result metadata for a database optimization pass.
///
/// Returned by [`Database::optimize`](crate::storage::Database::optimize).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptimizeStats {
    /// Database size in bytes before the pass.
    pub before_size_bytes: u64,
    /// Database size in bytes after the pass.
    pub after_size_bytes: u64,
    /// How long the pass took in milliseconds.
    pub elapsed_ms: u64,
    /// Whether `VACUUM` actually ran (it is skipped when there is nothing to reclaim or the
    /// database is busy).
    pub vacuum_ran: bool,
}

/// Mode for the cleanup command.
///
/// Passed to [`Database::cleanup_by_mode`](crate::storage::Database::cleanup_by_mode).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CleanupMode {
    /// Remove internal ggnmem / shell noise commands.
    Internal,
    /// Remove true duplicate rows (same content_hash), keeping the one with the lowest rowid.
    Duplicates,
    /// Remove commands that failed once and were never re-run.
    Failed,
    /// Remove commands older than N days.
    OlderThan(u32),
    /// Enforce retention policy: max age + max count.
    Retention {
        /// Remove commands older than this many days. `0` skips the age step.
        max_age_days: u32,
        /// Keep at most this many of the most recent commands. `0` skips the count step.
        max_commands: u64,
    },
}
