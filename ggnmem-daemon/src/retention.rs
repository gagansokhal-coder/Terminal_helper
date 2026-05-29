use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use tokio::sync::watch;
use tracing::{debug, info, warn};

use crate::error::DaemonResult;
use ggnmem_db::{CleanupStats, Database, DatabaseConfig};

/// Lightweight startup check: if (now - last_cleanup) >= interval, run cleanup immediately.
/// Called once, synchronously (via spawn_blocking), before the accept loop starts.
pub async fn startup_cleanup_if_overdue(
    database_path: &Path,
    interval_secs: u64,
    retention_days: u32,
    max_commands: u64,
) -> DaemonResult<Option<CleanupStats>> {
    let path = database_path.to_path_buf();

    tokio::task::spawn_blocking(move || -> ggnmem_db::DbResult<Option<CleanupStats>> {
        let db = Database::open(&DatabaseConfig::new(path))?;
        let last_cleanup = db.get_last_cleanup_at_ms()?;
        let now = ggnmem_db::time::unix_epoch_millis();
        let interval_ms = (interval_secs as i64) * 1000;

        if (now - last_cleanup) >= interval_ms {
            info!(
                elapsed_ms = now - last_cleanup,
                "startup cleanup is overdue, running now"
            );
            let stats = db.run_automatic_cleanup(retention_days, max_commands)?;
            info!(
                removed = stats.removed,
                remaining = stats.remaining,
                "startup cleanup completed"
            );
            Ok(Some(stats))
        } else {
            debug!(
                elapsed_ms = now - last_cleanup,
                interval_ms, "startup cleanup is not overdue"
            );
            Ok(None)
        }
    })
    .await?
    .map_err(Into::into)
}

/// Spawn a background task that runs cleanup every `interval` duration.
/// Returns the JoinHandle so the caller can abort it on shutdown.
/// Respects the shutdown receiver to exit cleanly.
pub fn spawn_periodic_cleanup(
    database_path: PathBuf,
    interval: Duration,
    retention_days: u32,
    max_commands: u64,
    mut shutdown_rx: watch::Receiver<bool>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        info!(
            "periodic cleanup scheduled every {} seconds",
            interval.as_secs()
        );
        let mut timer = tokio::time::interval(interval);
        // Skip the immediate first tick; startup cleanup handles overdue work.
        timer.tick().await;

        loop {
            tokio::select! {
                _ = timer.tick() => {
                    let path = database_path.clone();
                    let res = tokio::task::spawn_blocking(move || -> ggnmem_db::DbResult<()> {
                        let db = Database::open(&DatabaseConfig::new(path))?;
                        let stats = db.run_automatic_cleanup(retention_days, max_commands)?;
                        info!(removed = stats.removed, remaining = stats.remaining, "periodic cleanup completed");
                        Ok(())
                    }).await;

                    match res {
                        Ok(Err(e)) => warn!(%e, "periodic cleanup failed"),
                        Err(e) => warn!(%e, "periodic cleanup task failed to join"),
                        Ok(Ok(_)) => {}
                    }
                }
                changed = shutdown_rx.changed() => {
                    if changed.is_err() || *shutdown_rx.borrow() {
                        debug!("periodic cleanup task shutting down");
                        break;
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use ggnmem_db::{CommandId, NewCommand, NewSession, SessionId};
    use tempfile::NamedTempFile;

    use super::*;

    fn open_temp_database() -> (NamedTempFile, Database) {
        let temp = NamedTempFile::new().expect("temp db");
        let database = Database::open(&DatabaseConfig::new(temp.path().to_path_buf()))
            .expect("database opens");
        (temp, database)
    }

    fn insert_command(database: &Database, command: &str, completed_at_ms: i64) {
        let session_id = SessionId::new();
        database
            .insert_session(&NewSession {
                id: session_id.clone(),
                os_context: "linux".to_owned(),
                hostname: "devbox".to_owned(),
                shell: Some("zsh".to_owned()),
                started_at_ms: completed_at_ms - 10,
            })
            .expect("session inserted");

        database
            .insert_command(&NewCommand {
                id: CommandId::new(),
                session_id,
                command: command.to_owned(),
                cwd: "/workspace".to_owned(),
                exit_code: Some(0),
                duration_ms: Some(10),
                started_at_ms: Some(completed_at_ms - 10),
                completed_at_ms,
            })
            .expect("command inserted");
    }

    #[tokio::test]
    async fn test_startup_cleanup_when_overdue() {
        let (temp, database) = open_temp_database();
        let now = ggnmem_db::time::unix_epoch_millis();
        database
            .set_last_cleanup_at_ms(now - (48 * 60 * 60 * 1000))
            .expect("timestamp updates");
        insert_command(&database, "ggnmem recent", now - 1_000);
        insert_command(&database, "git status", now - 500);
        drop(database);

        let stats = startup_cleanup_if_overdue(temp.path(), 24 * 60 * 60, 365, 1_000_000)
            .await
            .expect("startup cleanup succeeds")
            .expect("cleanup was overdue");

        let database = Database::open(&DatabaseConfig::new(temp.path().to_path_buf()))
            .expect("database reopens");
        assert_eq!(stats.removed, 1);
        assert_eq!(stats.remaining, 1);
        assert!(database.get_last_cleanup_at_ms().expect("timestamp") >= now);
    }

    #[tokio::test]
    async fn test_startup_cleanup_when_not_overdue() {
        let (temp, database) = open_temp_database();
        let now = ggnmem_db::time::unix_epoch_millis();
        database
            .set_last_cleanup_at_ms(now - (60 * 60 * 1000))
            .expect("timestamp updates");
        drop(database);

        let stats = startup_cleanup_if_overdue(temp.path(), 24 * 60 * 60, 365, 1_000_000)
            .await
            .expect("startup cleanup succeeds");

        assert!(stats.is_none());
    }
}
