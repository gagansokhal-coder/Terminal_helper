use ggnmem_db::{CleanupMode, MatchKind};
use serde::{Deserialize, Serialize};

use crate::health::HealthStatus;

pub type ProtocolVersion = u16;
pub const PROTOCOL_VERSION: ProtocolVersion = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionPayload {
    pub session_id: String,
    pub os_context: String,
    pub hostname: String,
    pub shell: Option<String>,
    pub started_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandPayload {
    pub command_id: String,
    pub session_id: String,
    pub command: String,
    pub cwd: String,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<i64>,
    pub started_at_ms: Option<i64>,
    pub completed_at_ms: i64,
}

/// Lightweight command summary returned by query responses.
/// Avoids exposing internal IDs and hashes over IPC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSummary {
    pub command: String,
    pub cwd: String,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<i64>,
    pub completed_at_ms: i64,
    pub session_id: String,
}

/// A single search result returned over IPC.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResultSummary {
    pub command: String,
    pub cwd: String,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<i64>,
    pub completed_at_ms: i64,
    pub run_count: u64,
    pub match_kind: MatchKind,
    /// Composite score in [0.0, 1.0].
    pub score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DaemonRequest {
    Ping {
        version: ProtocolVersion,
    },
    Health {
        version: ProtocolVersion,
    },
    IngestCommand {
        version: ProtocolVersion,
        session: Box<SessionPayload>,
        command: Box<CommandPayload>,
    },
    Shutdown {
        version: ProtocolVersion,
    },
    QueryRecent {
        version: ProtocolVersion,
        limit: u32,
    },
    CountCommands {
        version: ProtocolVersion,
    },
    SearchCommands {
        version: ProtocolVersion,
        query: String,
        limit: u32,
        /// Optional cwd for result boosting.
        cwd: Option<String>,
        /// Sort by recency only, ignoring scoring weights.
        recent_only: bool,
    },
    CleanupCommands {
        version: ProtocolVersion,
        mode: CleanupMode,
    },
    OptimizeDb {
        version: ProtocolVersion,
    },
    GetDbStats {
        version: ProtocolVersion,
    },
    GetStats {
        version: ProtocolVersion,
    },
}

impl DaemonRequest {
    #[must_use]
    pub fn ping() -> Self {
        Self::Ping {
            version: PROTOCOL_VERSION,
        }
    }

    #[must_use]
    pub fn health() -> Self {
        Self::Health {
            version: PROTOCOL_VERSION,
        }
    }

    #[must_use]
    pub fn shutdown() -> Self {
        Self::Shutdown {
            version: PROTOCOL_VERSION,
        }
    }

    #[must_use]
    pub fn query_recent(limit: u32) -> Self {
        Self::QueryRecent {
            version: PROTOCOL_VERSION,
            limit,
        }
    }

    #[must_use]
    pub fn count_commands() -> Self {
        Self::CountCommands {
            version: PROTOCOL_VERSION,
        }
    }

    #[must_use]
    pub fn search_commands(query: impl Into<String>, limit: u32) -> Self {
        Self::SearchCommands {
            version: PROTOCOL_VERSION,
            query: query.into(),
            limit,
            cwd: None,
            recent_only: false,
        }
    }

    #[must_use]
    pub fn search_commands_with_options(
        query: impl Into<String>,
        limit: u32,
        cwd: Option<String>,
        recent_only: bool,
    ) -> Self {
        Self::SearchCommands {
            version: PROTOCOL_VERSION,
            query: query.into(),
            limit,
            cwd,
            recent_only,
        }
    }

    #[must_use]
    pub fn cleanup_commands() -> Self {
        Self::CleanupCommands {
            version: PROTOCOL_VERSION,
            mode: CleanupMode::Internal,
        }
    }

    #[must_use]
    pub fn cleanup_with_mode(mode: CleanupMode) -> Self {
        Self::CleanupCommands {
            version: PROTOCOL_VERSION,
            mode,
        }
    }

    #[must_use]
    pub fn optimize_db() -> Self {
        Self::OptimizeDb {
            version: PROTOCOL_VERSION,
        }
    }

    #[must_use]
    pub fn get_db_stats() -> Self {
        Self::GetDbStats {
            version: PROTOCOL_VERSION,
        }
    }

    #[must_use]
    pub fn get_stats() -> Self {
        Self::GetStats {
            version: PROTOCOL_VERSION,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonResponse {
    pub version: ProtocolVersion,
    pub kind: DaemonResponseKind,
}

impl DaemonResponse {
    #[must_use]
    pub fn pong() -> Self {
        Self {
            version: PROTOCOL_VERSION,
            kind: DaemonResponseKind::Pong,
        }
    }

    #[must_use]
    pub fn health(status: HealthStatus) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            kind: DaemonResponseKind::Health(status),
        }
    }

    #[must_use]
    pub fn accepted(queue_depth: usize) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            kind: DaemonResponseKind::Accepted { queue_depth },
        }
    }

    #[must_use]
    pub fn shutting_down() -> Self {
        Self {
            version: PROTOCOL_VERSION,
            kind: DaemonResponseKind::ShuttingDown,
        }
    }

    #[must_use]
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            kind: DaemonResponseKind::Error {
                code: code.into(),
                message: message.into(),
            },
        }
    }

    #[must_use]
    pub fn recent_commands(commands: Vec<CommandSummary>) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            kind: DaemonResponseKind::RecentCommands { commands },
        }
    }

    #[must_use]
    pub fn command_count(count: u64) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            kind: DaemonResponseKind::CommandCount { count },
        }
    }

    #[must_use]
    pub fn search_results(results: Vec<SearchResultSummary>) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            kind: DaemonResponseKind::SearchResults { results },
        }
    }

    #[must_use]
    pub fn cleanup_result(removed: u64, remaining: u64) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            kind: DaemonResponseKind::CleanupResult { removed, remaining },
        }
    }

    #[must_use]
    pub fn optimize_result(stats: ggnmem_db::OptimizeStats) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            kind: DaemonResponseKind::OptimizeResult { stats },
        }
    }

    #[must_use]
    pub fn db_stats_result(stats: ggnmem_db::DbStats) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            kind: DaemonResponseKind::DbStatsResult { stats },
        }
    }

    #[must_use]
    pub fn stats_result(stats: ggnmem_db::UsageStats, uptime_ms: u64) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            kind: DaemonResponseKind::StatsResult { stats, uptime_ms },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DaemonResponseKind {
    Pong,
    Health(HealthStatus),
    Accepted {
        queue_depth: usize,
    },
    ShuttingDown,
    Error {
        code: String,
        message: String,
    },
    RecentCommands {
        commands: Vec<CommandSummary>,
    },
    CommandCount {
        count: u64,
    },
    SearchResults {
        results: Vec<SearchResultSummary>,
    },
    CleanupResult {
        removed: u64,
        remaining: u64,
    },
    OptimizeResult {
        stats: ggnmem_db::OptimizeStats,
    },
    DbStatsResult {
        stats: ggnmem_db::DbStats,
    },
    StatsResult {
        stats: ggnmem_db::UsageStats,
        uptime_ms: u64,
    },
}
