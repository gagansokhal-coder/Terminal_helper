use thiserror::Error;

pub type DbResult<T> = Result<T, DbError>;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("database migration {version} failed")]
    Migration { version: i64 },
}
