use std::{path::Path, time::Duration};

use rusqlite::Connection;

use crate::{config::DatabaseConfig, error::DbResult, migrations};

pub const JOURNAL_MODE: &str = "wal";
pub const SYNCHRONOUS_MODE: &str = "normal";

pub fn open_database(config: &DatabaseConfig) -> DbResult<Connection> {
    let mut connection = Connection::open(&config.path)?;
    configure_connection(&connection, config)?;
    migrations::apply_migrations(&mut connection)?;
    Ok(connection)
}

pub fn open_database_at(path: impl AsRef<Path>) -> DbResult<Connection> {
    let config = DatabaseConfig::new(path.as_ref().to_path_buf());
    open_database(&config)
}

pub fn configure_connection(connection: &Connection, config: &DatabaseConfig) -> DbResult<()> {
    connection.busy_timeout(Duration::from_millis(config.busy_timeout_ms))?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "journal_mode", JOURNAL_MODE)?;
    connection.pragma_update(None, "synchronous", SYNCHRONOUS_MODE)?;
    connection.pragma_update(None, "mmap_size", config.mmap_size_bytes)?;
    // 8 MB page cache (negative value = KB).
    connection.pragma_update(None, "cache_size", -8000)?;
    // Keep temp tables in memory to avoid disk I/O.
    connection.pragma_update(None, "temp_store", "MEMORY")?;
    // Cap WAL file at 64 MB to prevent unbounded growth.
    connection.pragma_update(None, "journal_size_limit", 67_108_864_i64)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn configures_wal_and_normal_sync() {
        let temp = NamedTempFile::new().expect("temp db");
        let connection = open_database_at(temp.path()).expect("database opens");

        let journal_mode: String = connection
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .expect("journal mode");
        let synchronous: i64 = connection
            .pragma_query_value(None, "synchronous", |row| row.get(0))
            .expect("sync mode");

        assert_eq!(journal_mode, JOURNAL_MODE);
        assert_eq!(synchronous, 1);
    }
}
