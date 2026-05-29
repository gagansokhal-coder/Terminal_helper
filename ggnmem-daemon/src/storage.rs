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
