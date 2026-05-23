use std::{future::Future, sync::Arc, time::Instant};

use ggnmem_db::{time::unix_epoch_millis, DatabaseConfig};
use tokio::{sync::watch, task::JoinSet};
use tracing::{error, info, warn};

use crate::{
    config::DaemonConfig,
    error::DaemonResult,
    health::{HealthState, HealthStatus},
    ipc::{IpcConnection, IpcServer},
    protocol::{DaemonRequest, DaemonResponse},
    queue::{IngestionQueue, QueueCommand, QueueItem, QueueWorker},
    storage,
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
        storage::initialize_database(&self.config.database_path).await?;

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
        let result = self
            .accept_loop(listener, state, shutdown_rx, shutdown)
            .await;

        worker_handle.abort();
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
        DaemonRequest::CleanupCommands { .. } => {
            match storage::cleanup_commands(state.database_path.clone()).await {
                Ok(stats) => DaemonResponse::cleanup_result(stats.removed, stats.remaining),
                Err(error) => DaemonResponse::error("cleanup_failed", error.to_string()),
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
