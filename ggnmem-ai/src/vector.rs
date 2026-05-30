//! Vector storage layer for semantic search.
//!
//! Uses a **separate** SQLite database file (`vectors.db`) to keep
//! AI assets isolated from the core command history database.
//!
//! Designed for lazy initialization:
//! - `VectorStore::new()` does ZERO I/O
//! - `ensure_initialized()` creates the DB only when first needed
//! - When AI is disabled, no vector DB is ever created or opened
//!
//! sqlite-vec support:
//! - sqlite-vec is loaded as a **runtime extension** (not vendored)
//! - If sqlite-vec is not available, a fallback metadata table stores
//!   raw embedding bytes for future migration when the extension is installed
//! - The fallback table does NOT support true vector similarity search;
//!   it only stores embeddings for later use

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::error::{AiError, AiResult};

/// Number of embedding dimensions (matches target models).
pub const EMBEDDING_DIMENSIONS: usize = 384;

/// A vector similarity search result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorMatch {
    /// The command ID that produced this embedding.
    pub id: String,
    /// Distance from the query vector (lower = more similar).
    pub distance: f32,
}

/// Lazy-initialized vector storage backed by SQLite.
///
/// The vector DB is a separate file from the main ggnmem.db to ensure
/// AI assets don't affect core functionality.
pub struct VectorStore {
    /// Path to the vector database file.
    path: PathBuf,
    /// Whether initialization has been completed.
    initialized: AtomicBool,
    /// Whether sqlite-vec extension is available.
    has_sqlite_vec: AtomicBool,
}

impl VectorStore {
    /// Create a vector store pointing at the given path.
    ///
    /// **Does ZERO I/O.** The database is created lazily on first use
    /// via `ensure_initialized()`.
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            initialized: AtomicBool::new(false),
            has_sqlite_vec: AtomicBool::new(false),
        }
    }

    /// Check if the vector database file exists on disk.
    #[must_use]
    pub fn is_initialized(&self) -> bool {
        self.path.exists()
    }

    /// Check if sqlite-vec extension was successfully loaded.
    #[must_use]
    pub fn has_vec_extension(&self) -> bool {
        self.has_sqlite_vec.load(Ordering::Relaxed)
    }

    /// Get the path to the vector database.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Lazy initialization: create the database and tables if needed.
    ///
    /// This is the only method that performs I/O to create the DB.
    /// It tries to load the sqlite-vec extension; if unavailable,
    /// it falls back to a plain metadata table.
    pub fn ensure_initialized(&self) -> AiResult<()> {
        if self.initialized.load(Ordering::Relaxed) {
            return Ok(());
        }

        // Ensure parent directory exists.
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&self.path)?;
        configure_vector_db(&conn)?;

        // Attempt to load sqlite-vec extension.
        let vec_available = try_load_sqlite_vec(&conn);
        self.has_sqlite_vec.store(vec_available, Ordering::Relaxed);

        if vec_available {
            // Create the vec0 virtual table for vector similarity search.
            conn.execute_batch(&format!(
                "CREATE VIRTUAL TABLE IF NOT EXISTS command_vectors \
                 USING vec0(command_id TEXT PRIMARY KEY, embedding float[{EMBEDDING_DIMENSIONS}]);"
            ))?;
        }

        // Always create the metadata fallback table — stores raw embeddings
        // for installations without sqlite-vec, and acts as an index mapping
        // for installations with sqlite-vec.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS vector_meta (
                command_id TEXT PRIMARY KEY,
                dimensions INTEGER NOT NULL,
                embedding_bytes BLOB NOT NULL,
                created_at_ms INTEGER NOT NULL
            );",
        )?;

        self.initialized.store(true, Ordering::Relaxed);
        Ok(())
    }

    /// Insert an embedding for a command.
    ///
    /// Stores in both the vec0 table (if available) and the fallback meta table.
    pub fn insert(&self, command_id: &str, embedding: &[f32]) -> AiResult<()> {
        self.ensure_initialized()?;
        validate_dimensions(embedding)?;

        let conn = Connection::open(&self.path)?;
        let now_ms = ggnmem_db::time::unix_epoch_millis();
        let blob = floats_to_bytes(embedding);

        if self.has_sqlite_vec.load(Ordering::Relaxed) {
            // Insert into the vec0 virtual table.
            conn.execute(
                "INSERT OR REPLACE INTO command_vectors (command_id, embedding) VALUES (?1, ?2)",
                rusqlite::params![command_id, blob],
            )?;
        }

        // Always insert into the metadata table.
        conn.execute(
            "INSERT OR REPLACE INTO vector_meta (command_id, dimensions, embedding_bytes, created_at_ms) \
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![command_id, EMBEDDING_DIMENSIONS as i64, blob, now_ms],
        )?;

        Ok(())
    }

    /// Search for similar vectors using the vec0 extension.
    ///
    /// Falls back to an empty result set if sqlite-vec is not available.
    /// True semantic search requires the sqlite-vec extension.
    pub fn search(&self, query: &[f32], limit: usize) -> AiResult<Vec<VectorMatch>> {
        self.ensure_initialized()?;
        validate_dimensions(query)?;

        if !self.has_sqlite_vec.load(Ordering::Relaxed) {
            // Without sqlite-vec, we can't do vector similarity search.
            // Return empty — a future migration path will enable this.
            return Ok(Vec::new());
        }

        let conn = Connection::open(&self.path)?;
        let query_blob = floats_to_bytes(query);

        let mut stmt = conn.prepare(
            "SELECT command_id, distance \
             FROM command_vectors \
             WHERE embedding MATCH ?1 \
             ORDER BY distance \
             LIMIT ?2",
        )?;

        let matches = stmt
            .query_map(rusqlite::params![query_blob, limit as i64], |row| {
                Ok(VectorMatch {
                    id: row.get(0)?,
                    distance: row.get(1)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(matches)
    }

    /// Count the number of indexed vectors.
    pub fn count(&self) -> AiResult<u64> {
        if !self.is_initialized() {
            return Ok(0);
        }

        let conn = Connection::open(&self.path)?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM vector_meta",
            [],
            |row| row.get(0),
        )?;

        Ok(count as u64)
    }

    /// Delete a vector by command ID.
    pub fn delete(&self, command_id: &str) -> AiResult<()> {
        if !self.is_initialized() {
            return Ok(());
        }

        let conn = Connection::open(&self.path)?;

        if self.has_sqlite_vec.load(Ordering::Relaxed) {
            // vec0 tables use a special delete syntax.
            let _ = conn.execute(
                "DELETE FROM command_vectors WHERE command_id = ?1",
                rusqlite::params![command_id],
            );
        }

        conn.execute(
            "DELETE FROM vector_meta WHERE command_id = ?1",
            rusqlite::params![command_id],
        )?;

        Ok(())
    }
}

/// Configure the vector DB connection with appropriate pragmas.
fn configure_vector_db(conn: &Connection) -> AiResult<()> {
    conn.pragma_update(None, "journal_mode", "wal")?;
    conn.pragma_update(None, "synchronous", "normal")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    Ok(())
}

/// Try to detect the sqlite-vec extension at runtime.
///
/// Returns `true` if the extension is available.
/// Returns `false` if the extension is not available (lite install).
///
/// sqlite-vec can be made available by:
///   - Pre-loading via system-wide SQLite config
///   - Setting SQLITE_EXTENSIONS environment variable
///   - Future phase: loading from known paths
///     (e.g., ~/.local/lib/ggnmem/vec0.so)
fn try_load_sqlite_vec(conn: &Connection) -> bool {
    // Probe: try to use vec0 to detect if it's already loaded.
    // If sqlite-vec was pre-loaded system-wide, it will be available.
    //
    // Direct extension loading via Connection::load_extension() requires
    // unsafe code, which our workspace forbids (unsafe_code = "forbid").
    // A future phase can relax this for the ggnmem-ai crate specifically
    // to enable runtime loading from known paths.
    probe_sqlite_vec(conn)
}

/// Probe whether sqlite-vec is available by attempting a no-op query.
fn probe_sqlite_vec(conn: &Connection) -> bool {
    // Try creating a temporary vec0 table. If vec0 module is not registered,
    // this will fail with "no such module: vec0".
    let result = conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS _vec_probe USING vec0(v float[1]); \
         DROP TABLE IF EXISTS _vec_probe;",
    );
    result.is_ok()
}

/// Convert a slice of f32 to raw bytes (little-endian).
fn floats_to_bytes(floats: &[f32]) -> Vec<u8> {
    floats.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Validate that an embedding has the expected dimensions.
fn validate_dimensions(embedding: &[f32]) -> AiResult<()> {
    if embedding.len() != EMBEDDING_DIMENSIONS {
        return Err(AiError::EmbeddingFailed(format!(
            "expected {} dimensions, got {}",
            EMBEDDING_DIMENSIONS,
            embedding.len()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_embedding() -> Vec<f32> {
        vec![0.0_f32; EMBEDDING_DIMENSIONS]
    }

    #[test]
    fn new_does_no_io() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("vectors.db");
        let _store = VectorStore::new(path.clone());
        // The DB file should NOT exist yet.
        assert!(!path.exists());
    }

    #[test]
    fn ensure_initialized_creates_db() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("vectors.db");
        let store = VectorStore::new(path.clone());

        store.ensure_initialized().unwrap();
        assert!(path.exists());
        assert!(store.is_initialized());
    }

    #[test]
    fn ensure_initialized_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("vectors.db");
        let store = VectorStore::new(path);

        store.ensure_initialized().unwrap();
        store.ensure_initialized().unwrap();
        // Should not panic or error on second call.
    }

    #[test]
    fn insert_and_count() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("vectors.db");
        let store = VectorStore::new(path);

        store.ensure_initialized().unwrap();
        store.insert("cmd-1", &test_embedding()).unwrap();
        store.insert("cmd-2", &test_embedding()).unwrap();

        assert_eq!(store.count().unwrap(), 2);
    }

    #[test]
    fn insert_replaces_on_duplicate_id() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("vectors.db");
        let store = VectorStore::new(path);

        store.ensure_initialized().unwrap();
        store.insert("cmd-1", &test_embedding()).unwrap();
        store.insert("cmd-1", &test_embedding()).unwrap();

        assert_eq!(store.count().unwrap(), 1);
    }

    #[test]
    fn invalid_dimensions_rejected() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("vectors.db");
        let store = VectorStore::new(path);

        store.ensure_initialized().unwrap();
        let bad_embedding = vec![0.0_f32; 100]; // Wrong dimensions.
        let result = store.insert("cmd-1", &bad_embedding);
        assert!(result.is_err());
    }

    #[test]
    fn count_returns_zero_when_uninitialized() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("vectors.db");
        let store = VectorStore::new(path);

        // Not initialized — should return 0, not error.
        assert_eq!(store.count().unwrap(), 0);
    }

    #[test]
    fn search_without_vec_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("vectors.db");
        let store = VectorStore::new(path);

        store.ensure_initialized().unwrap();
        store.insert("cmd-1", &test_embedding()).unwrap();

        // Without sqlite-vec loaded, search returns empty.
        let results = store.search(&test_embedding(), 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn delete_removes_entry() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("vectors.db");
        let store = VectorStore::new(path);

        store.ensure_initialized().unwrap();
        store.insert("cmd-1", &test_embedding()).unwrap();
        store.insert("cmd-2", &test_embedding()).unwrap();
        assert_eq!(store.count().unwrap(), 2);

        store.delete("cmd-1").unwrap();
        assert_eq!(store.count().unwrap(), 1);
    }

    #[test]
    fn delete_on_uninitialized_is_noop() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("vectors.db");
        let store = VectorStore::new(path);

        // Should succeed silently even if DB doesn't exist.
        store.delete("cmd-1").unwrap();
    }

    #[test]
    fn floats_to_bytes_round_trip() {
        let values = vec![1.0_f32, -2.5, 3.14, 0.0];
        let bytes = floats_to_bytes(&values);
        assert_eq!(bytes.len(), values.len() * 4);

        // Verify round-trip.
        let recovered: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        assert_eq!(values, recovered);
    }
}
