use thiserror::Error;

pub type DaemonResult<T> = Result<T, DaemonError>;

#[derive(Debug, Error)]
pub enum DaemonError {
    #[error("configuration error: {0}")]
    InvalidConfig(String),

    #[error("XDG_RUNTIME_DIR is required for Linux IPC")]
    MissingRuntimeDir,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("database error: {0}")]
    Database(#[from] ggnmem_db::DbError),

    #[error("serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    #[error("IPC frame is too large: {0} bytes")]
    FrameTooLarge(usize),

    #[error("queue is full")]
    QueueFull,

    #[error("queue is closed")]
    QueueClosed,

    #[error("task join error: {0}")]
    Join(#[from] tokio::task::JoinError),
}
