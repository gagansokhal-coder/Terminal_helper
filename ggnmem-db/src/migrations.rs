use rusqlite::Connection;

use crate::error::{DbError, DbResult};

const MIGRATIONS: &[(i64, &str)] = &[(1, include_str!("../migrations/0001_initial.sql"))];

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
            1
        );
    }
}
