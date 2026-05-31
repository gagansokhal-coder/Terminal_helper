use std::path::PathBuf;

use ggnmem_db::{CommandId, Database, DatabaseConfig, NewCommand, NewSession, SessionId};

use crate::{
    error::DaemonResult,
    protocol::{CommandPayload, CommandSummary, SessionPayload},
    queue::{QueueCommand, QueueItem},
};

pub async fn initialize_database(path: &std::path::Path) -> DaemonResult<()> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let _database = Database::open(&DatabaseConfig::new(path))?;
        Ok::<(), ggnmem_db::DbError>(())
    })
    .await??;
    Ok(())
}

pub async fn persist_queue_item(database_path: PathBuf, item: QueueItem) -> DaemonResult<()> {
    match item {
        QueueItem::Command(command) => persist_command(database_path, command).await,
    }
}

async fn persist_command(database_path: PathBuf, item: QueueCommand) -> DaemonResult<()> {
    tokio::task::spawn_blocking(move || {
        let database = Database::open(&DatabaseConfig::new(database_path))?;
        database.insert_session(&session_from_payload(&item.session))?;
        database.insert_command(&command_from_payload(&item.command))?;
        Ok::<(), ggnmem_db::DbError>(())
    })
    .await??;
    Ok(())
}

fn session_from_payload(payload: &SessionPayload) -> NewSession {
    NewSession {
        id: SessionId::from_storage(payload.session_id.clone()),
        os_context: payload.os_context.clone(),
        hostname: payload.hostname.clone(),
        shell: payload.shell.clone(),
        started_at_ms: payload.started_at_ms,
    }
}

fn command_from_payload(payload: &CommandPayload) -> NewCommand {
    NewCommand {
        id: CommandId::from_storage(payload.command_id.clone()),
        session_id: SessionId::from_storage(payload.session_id.clone()),
        command: payload.command.clone(),
        cwd: payload.cwd.clone(),
        exit_code: payload.exit_code,
        duration_ms: payload.duration_ms,
        started_at_ms: payload.started_at_ms,
        completed_at_ms: payload.completed_at_ms,
    }
}

pub async fn query_recent_commands(
    database_path: PathBuf,
    limit: u32,
) -> DaemonResult<Vec<CommandSummary>> {
    let summaries = tokio::task::spawn_blocking(move || {
        let database = Database::open(&DatabaseConfig::new(database_path))?;
        let records = database.list_recent_commands(limit)?;
        let result: Vec<CommandSummary> = records
            .into_iter()
            .map(|r| CommandSummary {
                command: r.command,
                cwd: r.cwd,
                exit_code: r.exit_code,
                duration_ms: r.duration_ms,
                completed_at_ms: r.completed_at_ms,
                session_id: r.session_id.as_str().to_owned(),
            })
            .collect();
        Ok::<Vec<CommandSummary>, ggnmem_db::DbError>(result)
    })
    .await??;
    Ok(summaries)
}

pub async fn count_all_commands(database_path: PathBuf) -> DaemonResult<u64> {
    let count = tokio::task::spawn_blocking(move || {
        let database = Database::open(&DatabaseConfig::new(database_path))?;
        let count = database.count_commands()?;
        Ok::<u64, ggnmem_db::DbError>(count)
    })
    .await??;
    Ok(count)
}

pub async fn search_commands(
    database_path: PathBuf,
    query: String,
    limit: u32,
    cwd: Option<String>,
    recent_only: bool,
) -> DaemonResult<Vec<crate::protocol::SearchResultSummary>> {
    let results = tokio::task::spawn_blocking(move || {
        let database = Database::open(&DatabaseConfig::new(database_path))?;
        let mut opts = ggnmem_db::SearchOptions::new(&query).with_limit(limit);
        if let Some(c) = cwd {
            opts = opts.with_cwd(c);
        }
        opts = opts.with_recent_only(recent_only);
        let results = database.search_commands_v2(&opts)?;
        database.record_search_performed()?;
        let summaries: Vec<crate::protocol::SearchResultSummary> = results
            .into_iter()
            .map(|r| crate::protocol::SearchResultSummary {
                command: r.command,
                cwd: r.cwd,
                exit_code: r.exit_code,
                duration_ms: r.duration_ms,
                completed_at_ms: r.completed_at_ms,
                run_count: r.run_count,
                match_kind: r.match_kind,
                score: r.score,
            })
            .collect();
        Ok::<Vec<crate::protocol::SearchResultSummary>, ggnmem_db::DbError>(summaries)
    })
    .await??;
    Ok(results)
}

pub async fn cleanup_commands(
    database_path: PathBuf,
    mode: ggnmem_db::CleanupMode,
) -> DaemonResult<ggnmem_db::CleanupStats> {
    let stats = tokio::task::spawn_blocking(move || {
        let database = Database::open(&DatabaseConfig::new(database_path))?;
        database.cleanup_by_mode(&mode)
    })
    .await??;
    Ok(stats)
}

pub async fn optimize_database(database_path: PathBuf) -> DaemonResult<ggnmem_db::OptimizeStats> {
    let stats = tokio::task::spawn_blocking(move || {
        let database = Database::open(&DatabaseConfig::new(database_path))?;
        database.optimize()
    })
    .await??;
    Ok(stats)
}

pub async fn get_db_stats(database_path: PathBuf) -> DaemonResult<ggnmem_db::DbStats> {
    let stats = tokio::task::spawn_blocking(move || {
        let database = Database::open(&DatabaseConfig::new(database_path))?;
        database.db_stats()
    })
    .await??;
    Ok(stats)
}

pub async fn get_usage_stats(database_path: PathBuf) -> DaemonResult<ggnmem_db::UsageStats> {
    let stats = tokio::task::spawn_blocking(move || {
        let database = Database::open(&DatabaseConfig::new(database_path))?;
        database.usage_stats()
    })
    .await??;
    Ok(stats)
}

/// Run retention cleanup (used by the periodic scheduler and startup check).
pub async fn run_retention_cleanup(
    database_path: PathBuf,
    max_age_days: u32,
    max_commands: u64,
) -> DaemonResult<ggnmem_db::CleanupStats> {
    let stats = tokio::task::spawn_blocking(move || {
        let database = Database::open(&DatabaseConfig::new(database_path))?;
        database.run_automatic_cleanup(max_age_days, max_commands)
    })
    .await??;
    Ok(stats)
}

// ─── Phase 12B: Semantic + Hybrid Search ─────────────────────────────────────

/// Pure semantic search: embed query, search vector store, cross-reference
/// with the commands database for metadata.
pub async fn semantic_search(
    database_path: PathBuf,
    query: String,
    limit: u32,
) -> DaemonResult<Vec<crate::protocol::SemanticResultSummary>> {
    let results = tokio::task::spawn_blocking(move || {
        let ai_cfg = default_ai_config();
        let provider = Box::new(ggnmem_ai::TestEmbeddingProvider::new());
        let store = ggnmem_ai::VectorStore::new(ai_cfg.vector_db_path);
        let pipeline = ggnmem_ai::EmbeddingPipeline::new(provider, store);

        // Search vector store (embeds query internally).
        let matches = pipeline.search_embedding(&query, limit as usize + 10)?;

        // Cross-reference with commands DB for metadata.
        let database = Database::open(&DatabaseConfig::new(database_path))?;
        let mut summaries = Vec::with_capacity(matches.len());

        for m in &matches {
            if let Ok(Some(cmd)) = database.get_command_by_id(&m.id) {
                let similarity = 1.0 - m.distance as f64; // cosine sim = 1 - cosine dist
                summaries.push(crate::protocol::SemanticResultSummary {
                    command: cmd.command,
                    cwd: cmd.cwd,
                    exit_code: cmd.exit_code,
                    duration_ms: cmd.duration_ms,
                    completed_at_ms: cmd.completed_at_ms,
                    similarity: similarity.clamp(0.0, 1.0),
                });
            }
        }

        summaries.truncate(limit as usize);
        Ok::<Vec<crate::protocol::SemanticResultSummary>, ggnmem_ai::AiError>(summaries)
    })
    .await??;
    Ok(results)
}

/// Hybrid search: run FTS search and semantic search, then merge via RRF.
///
/// Reciprocal Rank Fusion formula:
///   `score_i = Σ weight_j / (k + rank_ij)` for each ranking list j.
///
/// This produces a unified ranking that benefits from both lexical
/// (FTS/fuzzy) and semantic (embedding) signals.
pub async fn hybrid_search_commands(
    database_path: PathBuf,
    query: String,
    limit: u32,
    cwd: Option<String>,
    recent_only: bool,
) -> DaemonResult<Vec<crate::protocol::SearchResultSummary>> {
    let results = tokio::task::spawn_blocking(move || {
        use crate::protocol::{FTS_WEIGHT, RRF_K, SEMANTIC_WEIGHT};
        use std::collections::HashMap;

        // 1. FTS search (existing).
        let database = Database::open(&DatabaseConfig::new(database_path.clone()))?;
        let mut opts = ggnmem_db::SearchOptions::new(&query).with_limit(limit * 2);
        if let Some(ref c) = cwd {
            opts = opts.with_cwd(c.clone());
        }
        opts = opts.with_recent_only(recent_only);
        let fts_results = database.search_commands_v2(&opts)?;
        database.record_search_performed()?;

        // 2. Semantic search.
        let ai_cfg = default_ai_config();
        let provider = Box::new(ggnmem_ai::TestEmbeddingProvider::new());
        let store = ggnmem_ai::VectorStore::new(ai_cfg.vector_db_path);
        let pipeline = ggnmem_ai::EmbeddingPipeline::new(provider, store);

        let semantic_matches = pipeline
            .search_embedding(&query, (limit * 2) as usize)
            .unwrap_or_default();

        // If no semantic results, return FTS only.
        if semantic_matches.is_empty() {
            let summaries: Vec<crate::protocol::SearchResultSummary> = fts_results
                .into_iter()
                .take(limit as usize)
                .map(|r| crate::protocol::SearchResultSummary {
                    command: r.command,
                    cwd: r.cwd,
                    exit_code: r.exit_code,
                    duration_ms: r.duration_ms,
                    completed_at_ms: r.completed_at_ms,
                    run_count: r.run_count,
                    match_kind: r.match_kind,
                    score: r.score,
                })
                .collect();
            return Ok::<Vec<crate::protocol::SearchResultSummary>, ggnmem_db::DbError>(summaries);
        }

        // 3. Look up semantic match metadata.
        let semantic_db = Database::open(&DatabaseConfig::new(database_path))?;
        let mut semantic_details: Vec<(ggnmem_db::CommandRecord, f32)> = Vec::new();
        for m in &semantic_matches {
            if let Ok(Some(cmd)) = semantic_db.get_command_by_id(&m.id) {
                semantic_details.push((cmd, m.distance));
            }
        }

        // 4. RRF merge.
        // Key: (command, cwd) pair to identify unique results.
        struct RrfEntry {
            command: String,
            cwd: String,
            exit_code: Option<i32>,
            duration_ms: Option<i64>,
            completed_at_ms: i64,
            run_count: u64,
            match_kind: ggnmem_db::MatchKind,
            rrf_score: f64,
        }

        let mut merged: HashMap<(String, String), RrfEntry> = HashMap::new();

        // Add FTS results with RRF scoring.
        for (rank, r) in fts_results.iter().enumerate() {
            let key = (r.command.clone(), r.cwd.clone());
            let rrf = FTS_WEIGHT as f64 / (RRF_K as f64 + rank as f64 + 1.0);
            let entry = merged.entry(key).or_insert_with(|| RrfEntry {
                command: r.command.clone(),
                cwd: r.cwd.clone(),
                exit_code: r.exit_code,
                duration_ms: r.duration_ms,
                completed_at_ms: r.completed_at_ms,
                run_count: r.run_count,
                match_kind: r.match_kind,
                rrf_score: 0.0,
            });
            entry.rrf_score += rrf;
        }

        // Add semantic results with RRF scoring.
        for (rank, (cmd, _distance)) in semantic_details.iter().enumerate() {
            let key = (cmd.command.clone(), cmd.cwd.clone());
            let rrf = SEMANTIC_WEIGHT as f64 / (RRF_K as f64 + rank as f64 + 1.0);
            let entry = merged.entry(key).or_insert_with(|| RrfEntry {
                command: cmd.command.clone(),
                cwd: cmd.cwd.clone(),
                exit_code: cmd.exit_code,
                duration_ms: cmd.duration_ms,
                completed_at_ms: cmd.completed_at_ms,
                run_count: 1,
                match_kind: ggnmem_db::MatchKind::Partial,
                rrf_score: 0.0,
            });
            entry.rrf_score += rrf;
        }

        // Sort by RRF score descending.
        let mut sorted: Vec<RrfEntry> = merged.into_values().collect();
        sorted.sort_by(|a, b| {
            b.rrf_score
                .partial_cmp(&a.rrf_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.truncate(limit as usize);

        // Normalize scores to [0.0, 1.0].
        let max_score = sorted.first().map(|e| e.rrf_score).unwrap_or(1.0);
        let summaries: Vec<crate::protocol::SearchResultSummary> = sorted
            .into_iter()
            .map(|e| crate::protocol::SearchResultSummary {
                command: e.command,
                cwd: e.cwd,
                exit_code: e.exit_code,
                duration_ms: e.duration_ms,
                completed_at_ms: e.completed_at_ms,
                run_count: e.run_count,
                match_kind: e.match_kind,
                score: if max_score > 0.0 {
                    e.rrf_score / max_score
                } else {
                    0.0
                },
            })
            .collect();

        Ok::<Vec<crate::protocol::SearchResultSummary>, ggnmem_db::DbError>(summaries)
    })
    .await??;
    Ok(results)
}

/// Build a default `AiConfig` from XDG paths.
fn default_ai_config() -> ggnmem_ai::AiConfig {
    ggnmem_ai::AiConfig::default()
}
