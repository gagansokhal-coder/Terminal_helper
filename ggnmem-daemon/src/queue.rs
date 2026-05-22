use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::{
    error::{DaemonError, DaemonResult},
    protocol::{CommandPayload, SessionPayload},
    storage,
};

#[derive(Debug, Clone)]
pub enum QueueItem {
    Command(QueueCommand),
}

#[derive(Debug, Clone)]
pub struct QueueCommand {
    pub session: SessionPayload,
    pub command: CommandPayload,
    pub attempts: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueReceipt {
    pub queue_depth: usize,
}

#[derive(Clone)]
pub struct IngestionQueue {
    sender: mpsc::Sender<QueueItem>,
    depth: Arc<AtomicUsize>,
    capacity: usize,
}

impl IngestionQueue {
    #[must_use]
    pub fn bounded(capacity: usize, max_retries: u8) -> (Self, IngestionReceiver) {
        let capacity = capacity.max(1);
        let (sender, receiver) = mpsc::channel(capacity);
        let depth = Arc::new(AtomicUsize::new(0));
        (
            Self {
                sender: sender.clone(),
                depth: Arc::clone(&depth),
                capacity,
            },
            IngestionReceiver {
                receiver,
                sender,
                depth,
                capacity,
                max_retries,
            },
        )
    }

    pub fn try_enqueue(&self, item: QueueItem) -> DaemonResult<QueueReceipt> {
        match self.sender.try_send(item) {
            Ok(()) => {
                let queue_depth = self.depth.fetch_add(1, Ordering::AcqRel) + 1;
                Ok(QueueReceipt { queue_depth })
            }
            Err(mpsc::error::TrySendError::Full(_)) => Err(DaemonError::QueueFull),
            Err(mpsc::error::TrySendError::Closed(_)) => Err(DaemonError::QueueClosed),
        }
    }

    #[must_use]
    pub fn depth(&self) -> usize {
        self.depth.load(Ordering::Acquire)
    }

    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

pub struct IngestionReceiver {
    receiver: mpsc::Receiver<QueueItem>,
    sender: mpsc::Sender<QueueItem>,
    depth: Arc<AtomicUsize>,
    capacity: usize,
    max_retries: u8,
}

impl IngestionReceiver {
    async fn recv(&mut self) -> Option<QueueItem> {
        let item = self.receiver.recv().await;
        if item.is_some() {
            self.depth.fetch_sub(1, Ordering::AcqRel);
        }
        item
    }

    fn retry(&self, item: QueueItem) -> DaemonResult<()> {
        let item = match item {
            QueueItem::Command(mut command) => {
                if command.attempts >= self.max_retries {
                    return Err(DaemonError::QueueClosed);
                }
                command.attempts += 1;
                QueueItem::Command(command)
            }
        };

        match self.sender.try_send(item) {
            Ok(()) => {
                self.depth.fetch_add(1, Ordering::AcqRel);
                Ok(())
            }
            Err(mpsc::error::TrySendError::Full(_)) => Err(DaemonError::QueueFull),
            Err(mpsc::error::TrySendError::Closed(_)) => Err(DaemonError::QueueClosed),
        }
    }

    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

pub struct QueueWorker {
    receiver: IngestionReceiver,
    database_path: PathBuf,
}

impl QueueWorker {
    #[must_use]
    pub fn new(receiver: IngestionReceiver, database_path: PathBuf) -> Self {
        Self {
            receiver,
            database_path,
        }
    }

    pub async fn run(mut self) {
        info!(
            capacity = self.receiver.capacity(),
            "ingestion queue worker started"
        );
        while let Some(item) = self.receiver.recv().await {
            let retry_item = item.clone();
            match storage::persist_queue_item(self.database_path.clone(), item).await {
                Ok(()) => {}
                Err(error) => {
                    warn!(%error, "failed to persist queue item");
                    if let Err(retry_error) = self.receiver.retry(retry_item) {
                        warn!(%retry_error, "queue item dropped after retry failure");
                    }
                }
            }
        }
        info!("ingestion queue worker stopped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_queue_reports_overflow() {
        let (queue, _receiver) = IngestionQueue::bounded(1, 1);
        let item = QueueItem::Command(QueueCommand {
            session: SessionPayload {
                session_id: "s".to_owned(),
                os_context: "linux".to_owned(),
                hostname: "host".to_owned(),
                shell: Some("zsh".to_owned()),
                started_at_ms: 1,
            },
            command: CommandPayload {
                command_id: "c".to_owned(),
                session_id: "s".to_owned(),
                command: "git status".to_owned(),
                cwd: "/tmp".to_owned(),
                exit_code: Some(0),
                duration_ms: Some(1),
                started_at_ms: Some(1),
                completed_at_ms: 2,
            },
            attempts: 0,
        });

        assert!(queue.try_enqueue(item.clone()).is_ok());
        assert!(matches!(
            queue.try_enqueue(item),
            Err(DaemonError::QueueFull)
        ));
    }
}
