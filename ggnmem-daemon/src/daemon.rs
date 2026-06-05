use fs2::FileExt;
use std::{
    fs,
    future::Future,
    io,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use ggnmem_db::{time::unix_epoch_millis, DatabaseConfig};
use tokio::{sync::watch, task::JoinSet};
use tracing::{error, info, warn};

use crate::{
    config::DaemonConfig,
    error::{DaemonError, DaemonResult},
    health::{HealthState, HealthStatus},
    ipc::{IpcConnection, IpcServer},
    protocol::{DaemonRequest, DaemonResponse},
    queue::{IngestionQueue, QueueCommand, QueueItem, QueueWorker},
    retention, storage,
};

pub struct Daemon {
    config: DaemonConfig,
}

impl Daemon {
    #[must_use]
    pub fn new(config: DaemonConfig) -> Self {
        Self { config }
    }

    pub async fn run_until_ctrl_c(self) -> DaemonResult<()> {
        self.run_until_shutdown(async {
            if let Err(error) = tokio::signal::ctrl_c().await {
                warn!(%error, "failed to wait for ctrl-c");
            }
        })
        .await
    }

    pub async fn run_until_shutdown<F>(self, shutdown: F) -> DaemonResult<()>
    where
        F: Future<Output = ()> + Send,
    {
        info!("starting ggnmem daemon");
        ensure_parent_dirs(&self.config)?;

        // Acquire an exclusive advisory lock on the PID file.
        // The returned File handle must stay alive for the daemon's lifetime;
        // dropping it releases the lock automatically.
        let pid_path = state_pid_path();
        let _lock_guard: Option<fs::File> = match pid_path {
            Some(ref path) => Some(acquire_pid_lock(path)?),
            None => None,
        };

        storage::initialize_database(&self.config.database_path).await?;

        if self.config.cleanup_enabled {
            match retention::startup_cleanup_if_overdue(
                &self.config.database_path,
                self.config.cleanup_interval_secs,
                self.config.retention_days,
                self.config.max_commands,
            )
            .await
            {
                Ok(Some(stats)) => info!(
                    removed = stats.removed,
                    remaining = stats.remaining,
                    "startup cleanup completed"
                ),
                Ok(None) => info!("startup cleanup: not overdue, skipped"),
                Err(e) => warn!(%e, "startup cleanup failed (non-fatal)"),
            }
        }

        let listener = IpcServer::bind(&self.config.endpoint).await?;
        let (queue, receiver) =
            IngestionQueue::bounded(self.config.queue_capacity, self.config.max_retries);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let state = Arc::new(DaemonState {
            started_at: Instant::now(),
            queue,
            database_path: self.config.database_path.clone(),
            shutdown_tx,
            platform: self.config.endpoint.platform_name().to_owned(),
        });

        let worker = QueueWorker::new(receiver, self.config.database_path.clone());
        let worker_handle = tokio::spawn(worker.run());

        let cleanup_handle = if self.config.cleanup_enabled {
            Some(retention::spawn_periodic_cleanup(
                self.config.database_path.clone(),
                Duration::from_secs(self.config.cleanup_interval_secs),
                self.config.retention_days,
                self.config.max_commands,
                shutdown_rx.clone(),
            ))
        } else {
            None
        };

        let result = self
            .accept_loop(listener, state, shutdown_rx, shutdown)
            .await;

        worker_handle.abort();
        if let Some(handle) = cleanup_handle {
            handle.abort();
        }

        // Clean up PID file on graceful shutdown.
        // The lock is released when _lock_guard is dropped at scope exit.
        if let Some(ref path) = pid_path {
            let _ = fs::remove_file(path);
            info!("removed PID file");
        }

        info!("ggnmem daemon stopped");
        result
    }

    async fn accept_loop<F>(
        &self,
        mut listener: IpcServer,
        state: Arc<DaemonState>,
        mut shutdown_rx: watch::Receiver<bool>,
        shutdown: F,
    ) -> DaemonResult<()>
    where
        F: Future<Output = ()> + Send,
    {
        let mut connections = JoinSet::new();
        tokio::pin!(shutdown);

        loop {
            tokio::select! {
                () = &mut shutdown => {
                    info!("external shutdown requested");
                    break;
                }
                changed = shutdown_rx.changed() => {
                    if changed.is_ok() && *shutdown_rx.borrow() {
                        info!("internal shutdown requested");
                        break;
                    }
                }
                accepted = listener.accept() => {
                    match accepted {
                        Ok(connection) => {
                            let state = Arc::clone(&state);
                            connections.spawn(async move {
                                if let Err(error) = handle_connection(connection, state).await {
                                    warn!(%error, "IPC connection failed");
                                }
                            });
                        }
                        Err(error) => {
                            warn!(%error, "IPC accept failed");
                        }
                    }
                }
                Some(joined) = connections.join_next(), if !connections.is_empty() => {
                    if let Err(error) = joined {
                        warn!(%error, "IPC handler task failed");
                    }
                }
            }
        }

        listener.shutdown().await?;

        while let Some(joined) = connections.join_next().await {
            if let Err(error) = joined {
                warn!(%error, "IPC handler task failed during shutdown");
            }
        }

        Ok(())
    }
}

struct DaemonState {
    started_at: Instant,
    queue: IngestionQueue,
    database_path: std::path::PathBuf,
    shutdown_tx: watch::Sender<bool>,
    platform: String,
}

impl DaemonState {
    fn health(&self) -> HealthStatus {
        HealthStatus {
            state: HealthState::Running,
            uptime_ms: self
                .started_at
                .elapsed()
                .as_millis()
                .min(u128::from(u64::MAX)) as u64,
            queue_depth: self.queue.depth(),
            queue_capacity: self.queue.capacity(),
            db_connected: self.database_path.exists(),
            platform: self.platform.clone(),
            checked_at_ms: unix_epoch_millis(),
        }
    }
}

async fn handle_connection(
    mut connection: IpcConnection,
    state: Arc<DaemonState>,
) -> DaemonResult<()> {
    let request: DaemonRequest = connection.receive().await?;
    let response = match request {
        DaemonRequest::Ping { .. } => DaemonResponse::pong(),
        DaemonRequest::Health { .. } => DaemonResponse::health(state.health()),
        DaemonRequest::Shutdown { .. } => {
            let response = DaemonResponse::shutting_down();
            if state.shutdown_tx.send(true).is_err() {
                error!("failed to notify daemon shutdown");
            }
            response
        }
        DaemonRequest::IngestCommand {
            session, command, ..
        } => {
            // Pre-ingestion filter: silently drop noise commands.
            if !ggnmem_db::should_ingest(&command.command) {
                DaemonResponse::accepted(state.queue.depth())
            } else {
                match state.queue.try_enqueue(QueueItem::Command(QueueCommand {
                    session: *session,
                    command: *command,
                    attempts: 0,
                })) {
                    Ok(receipt) => DaemonResponse::accepted(receipt.queue_depth),
                    Err(error) => DaemonResponse::error("queue_unavailable", error.to_string()),
                }
            }
        }
        DaemonRequest::QueryRecent { limit, .. } => {
            match storage::query_recent_commands(state.database_path.clone(), limit).await {
                Ok(commands) => DaemonResponse::recent_commands(commands),
                Err(error) => DaemonResponse::error("query_failed", error.to_string()),
            }
        }
        DaemonRequest::CountCommands { .. } => {
            match storage::count_all_commands(state.database_path.clone()).await {
                Ok(count) => DaemonResponse::command_count(count),
                Err(error) => DaemonResponse::error("count_failed", error.to_string()),
            }
        }
        DaemonRequest::SearchCommands {
            query,
            limit,
            cwd,
            recent_only,
            ..
        } => {
            match storage::search_commands(
                state.database_path.clone(),
                query,
                limit,
                cwd,
                recent_only,
            )
            .await
            {
                Ok(results) => DaemonResponse::search_results(results),
                Err(error) => DaemonResponse::error("search_failed", error.to_string()),
            }
        }
        DaemonRequest::CleanupCommands { mode, .. } => {
            match storage::cleanup_commands(state.database_path.clone(), mode).await {
                Ok(stats) => DaemonResponse::cleanup_result(stats.removed, stats.remaining),
                Err(error) => DaemonResponse::error("cleanup_failed", error.to_string()),
            }
        }
        DaemonRequest::OptimizeDb { .. } => {
            match storage::optimize_database(state.database_path.clone()).await {
                Ok(stats) => DaemonResponse::optimize_result(stats),
                Err(error) => DaemonResponse::error("optimize_failed", error.to_string()),
            }
        }
        DaemonRequest::GetDbStats { .. } => {
            match storage::get_db_stats(state.database_path.clone()).await {
                Ok(stats) => DaemonResponse::db_stats_result(stats),
                Err(error) => DaemonResponse::error("db_stats_failed", error.to_string()),
            }
        }
        DaemonRequest::GetStats { .. } => {
            let uptime_ms = state.started_at.elapsed().as_millis() as u64;
            match storage::get_usage_stats(state.database_path.clone()).await {
                Ok(stats) => DaemonResponse::stats_result(stats, uptime_ms),
                Err(error) => DaemonResponse::error("stats_failed", error.to_string()),
            }
        }
        DaemonRequest::SemanticSearch { query, limit, .. } => {
            match storage::semantic_search(state.database_path.clone(), query, limit).await {
                Ok(results) => DaemonResponse::semantic_results(results),
                Err(error) => DaemonResponse::error("semantic_search_failed", error.to_string()),
            }
        }
    };

    connection.send(&response).await?;
    connection.shutdown().await?;
    Ok(())
}

fn ensure_parent_dirs(config: &DaemonConfig) -> DaemonResult<()> {
    if let Some(parent) = config.database_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    config.endpoint.prepare_parent_dir()?;
    Ok(())
}

pub async fn run_loaded_config() -> DaemonResult<()> {
    crate::logging::init_logging();
    Daemon::new(DaemonConfig::load()?).run_until_ctrl_c().await
}

pub fn database_config_for_path(path: std::path::PathBuf) -> DatabaseConfig {
    DatabaseConfig::new(path)
}

// ─── PID file + advisory lock ────────────────────────────────────────────────

/// Get the path to `~/.local/state/ggnmem/daemon.pid`.
fn state_pid_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from).map(|home| {
        home.join(".local")
            .join("state")
            .join("ggnmem")
            .join("daemon.pid")
    })
}

/// Acquire an exclusive advisory lock on the PID file.
///
/// * If the lock succeeds, the current PID is written to the file and the
///   open `File` handle is returned.  The caller **must** keep this handle
///   alive for the entire daemon lifetime — dropping it releases the lock.
/// * If another process already holds the lock (`WouldBlock`), this returns
///   an error telling the user to stop the existing daemon first.
fn acquire_pid_lock(pid_path: &Path) -> DaemonResult<fs::File> {
    if let Some(parent) = pid_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(pid_path)?;

    match file.try_lock_exclusive() {
        Ok(()) => {
            // Lock acquired — write our PID.
            let pid = std::process::id();
            // Truncate any previous content and write new PID.
            file.set_len(0)?;
            io::Write::write_all(&mut &file, pid.to_string().as_bytes())?;
            info!(pid, path = %pid_path.display(), "acquired PID lock");
            Ok(file)
        }
        Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
            // Another daemon holds the lock.  Try to read its PID for
            // a friendlier error message.
            let existing_pid = fs::read_to_string(pid_path).unwrap_or_default();
            let existing_pid = existing_pid.trim();
            let msg = if existing_pid.is_empty() {
                "another ggnmem daemon is already running. \
                 Stop it with `ggnmem stop` before starting a new one."
                    .to_owned()
            } else {
                format!(
                    "another ggnmem daemon is already running (PID {existing_pid}). \
                     Stop it with `ggnmem stop` before starting a new one."
                )
            };
            Err(DaemonError::InvalidConfig(msg))
        }
        Err(e) => Err(DaemonError::Io(e)),
    }
}
