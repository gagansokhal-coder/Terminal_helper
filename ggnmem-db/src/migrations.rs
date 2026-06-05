use rusqlite::Connection;

use crate::error::{DbError, DbResult};

const MIGRATIONS: &[(i64, &str)] = &[
    (1, include_str!("../migrations/0001_initial.sql")),
    (2, include_str!("../migrations/0002_retention_meta.sql")),
    (3, include_str!("../migrations/0003_maintenance_meta.sql")),
    (4, include_str!("../migrations/0004_search_metrics.sql")),
];

pub fn apply_migrations(connection: &mut Connection) -> DbResult<()> {
    let current_version: i64 =
        connection.pragma_query_value(None, "user_version", |row| row.get(0))?;

    for (version, sql) in MIGRATIONS {
        if *version <= current_version {
            continue;
        }

        let tx = connection.transaction()?;
        tx.execute_batch(sql)
            .map_err(|_| DbError::Migration { version: *version })?;
        tx.pragma_update(None, "user_version", version)?;
        tx.commit()?;
    }

    Ok(())
}

pub fn current_schema_version(connection: &Connection) -> DbResult<i64> {
    Ok(connection.pragma_query_value(None, "user_version", |row| row.get(0))?)
}

#[cfg(test)]
mod tests {
    use tempfile::NamedTempFile;

    use crate::connection::open_database_at;

    use super::*;

    #[test]
    fn migrations_are_idempotent() {
        let temp = NamedTempFile::new().expect("temp db");
        let mut connection = open_database_at(temp.path()).expect("database opens");

        apply_migrations(&mut connection).expect("second migration pass");

        assert_eq!(
            current_schema_version(&connection).expect("schema version"),
            4
        );
    }

    #[test]
    fn test_migration_0002_idempotent() {
        let temp = NamedTempFile::new().expect("temp db");
        let mut connection = open_database_at(temp.path()).expect("database opens");

        apply_migrations(&mut connection).expect("migration pass succeeds");

        let last_cleanup_at_ms: i64 = connection
            .query_row(
                "SELECT last_cleanup_at_ms FROM retention_meta WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .expect("retention meta row exists");

        assert_eq!(last_cleanup_at_ms, 0);
        assert_eq!(
            current_schema_version(&connection).expect("schema version"),
            4
        );
    }

    #[test]
    fn test_migration_0003_maintenance_defaults() {
        let temp = NamedTempFile::new().expect("temp db");
        let connection = open_database_at(temp.path()).expect("database opens");

        let values: (i64, i64, i64, i64, i64) = connection
            .query_row(
                r#"
                SELECT
                    last_optimize_at_ms,
                    last_cleanup_at_ms,
                    last_cleanup_removed,
                    last_cleanup_remaining,
                    searches_performed
                FROM maintenance_meta
                WHERE id = 1
                "#,
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("maintenance meta row exists");

        assert_eq!(values, (0, 0, 0, 0, 0));
    }
}
