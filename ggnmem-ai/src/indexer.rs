//! Background indexing orchestrator for semantic search.
//!
//! Reads command history from the core `ggnmem.db`, determines which
//! commands are not yet indexed in the vector store, and embeds them
//! in batches through the `EmbeddingPipeline`.
//!
//! The indexer is synchronous and CLI-driven (not daemon-hosted).
//! It is designed to run in the foreground of `ggnmem ai reindex`
//! while the daemon continues accepting new commands.

use std::path::Path;

use ggnmem_db::{Database, DatabaseConfig};

use crate::embedding::EmbeddingPipeline;
use crate::error::AiResult;

/// Progress state for the background indexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexProgress {
    /// Total commands in the database.
    pub total: u64,
    /// Number of commands that have embeddings in the vector store.
    pub indexed: u64,
    /// Timestamp (ms since epoch) of the last indexing run.
    pub last_run_ms: i64,
}

impl IndexProgress {
    /// Percentage of commands indexed (0–100).
    #[must_use]
    pub fn percent(&self) -> u64 {
        (self.indexed * 100).checked_div(self.total).unwrap_or(100)
    }

    /// Whether all commands are indexed.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.indexed >= self.total
    }
}

/// Maximum number of commands per batch to limit peak memory.
const BATCH_SIZE: usize = 100;

/// Get the current indexing progress without performing any indexing.
///
/// Compares command count in `ggnmem.db` against vector count in the
/// vector store to determine how many commands are un-indexed.
pub fn get_index_progress(db_path: &Path, pipeline: &EmbeddingPipeline) -> AiResult<IndexProgress> {
    let db = Database::open(&DatabaseConfig::new(db_path.to_path_buf()))?;
    let total = db.count_commands()?;
    let indexed = pipeline.vector_count()?;

    Ok(IndexProgress {
        total,
        indexed,
        last_run_ms: 0, // Caller can fill from config/metadata if needed.
    })
}

/// Index all un-indexed commands from the database.
///
/// This is the main entry point for `ggnmem ai reindex`.
///
/// 1. Reads all command (id, text) pairs from `ggnmem.db`.
/// 2. Checks which IDs already exist in the vector store.
/// 3. Embeds un-indexed commands in batches of `BATCH_SIZE`.
/// 4. Calls `progress` after each embedded command.
///
/// Returns the total number of newly indexed commands.
pub fn index_all_commands(
    db_path: &Path,
    pipeline: &EmbeddingPipeline,
    mut progress: impl FnMut(u64, u64),
) -> AiResult<u64> {
    let db = Database::open(&DatabaseConfig::new(db_path.to_path_buf()))?;

    // 1. Get all command IDs + text from the main database.
    let all_commands = db.list_commands_for_indexing()?;

    // 2. Get already-indexed IDs from the vector store.
    let indexed_ids = pipeline.indexed_ids()?;

    // 3. Filter to un-indexed commands.
    let to_index: Vec<(String, String)> = all_commands
        .into_iter()
        .filter(|(id, _)| !indexed_ids.contains(id))
        .collect();

    let total = to_index.len() as u64;
    if total == 0 {
        progress(0, 0);
        return Ok(0);
    }

    // 4. Index in batches.
    let mut total_indexed = 0u64;
    for batch in to_index.chunks(BATCH_SIZE) {
        let batch_vec: Vec<(String, String)> = batch.to_vec();
        let batch_count = pipeline.batch_index(&batch_vec, |done, _batch_total| {
            progress(total_indexed + done, total);
        })?;
        total_indexed += batch_count;
    }

    Ok(total_indexed)
}

/// Reindex all commands: delete existing embeddings, then re-embed everything.
///
/// This is the entry point for a full rebuild.
pub fn reindex_all_commands(
    db_path: &Path,
    pipeline: &EmbeddingPipeline,
    progress: impl FnMut(u64, u64),
) -> AiResult<u64> {
    // Delete all existing embeddings.
    pipeline.delete_all_embeddings()?;

    // Re-index everything.
    index_all_commands(db_path, pipeline, progress)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::TestEmbeddingProvider;
    use crate::vector::VectorStore;
    use ggnmem_db::{CommandId, NewCommand, NewSession, SessionId};
    use tempfile::TempDir;

    /// Helper: create a test database with N commands.
    fn setup_test_db(tmp: &TempDir, count: usize) -> std::path::PathBuf {
        let db_path = tmp.path().join("ggnmem.db");
        let db = Database::open(&DatabaseConfig::new(db_path.clone())).unwrap();

        let session = NewSession {
            id: SessionId::new(),
            os_context: "linux".to_owned(),
            hostname: "test".to_owned(),
            shell: Some("bash".to_owned()),
            started_at_ms: 1000,
        };
        db.insert_session(&session).unwrap();

        for i in 0..count {
            let cmd = NewCommand {
                id: CommandId::new(),
                session_id: session.id.clone(),
                command: format!("command-{i}"),
                cwd: "/home/test".to_owned(),
                exit_code: Some(0),
                duration_ms: Some(100),
                started_at_ms: Some(1000 + i as i64),
                completed_at_ms: 2000 + i as i64,
            };
            db.insert_command(&cmd).unwrap();
        }

        db_path
    }

    #[test]
    fn test_index_progress_empty_db() {
        let tmp = TempDir::new().unwrap();
        let db_path = setup_test_db(&tmp, 0);
        let store = VectorStore::new(tmp.path().join("vectors.db"));
        let provider = Box::new(TestEmbeddingProvider::new());
        let pipeline = EmbeddingPipeline::new(provider, store);

        let progress = get_index_progress(&db_path, &pipeline).unwrap();
        assert_eq!(progress.total, 0);
        assert_eq!(progress.indexed, 0);
        assert!(progress.is_complete());
    }

    #[test]
    fn test_index_all_commands() {
        let tmp = TempDir::new().unwrap();
        let db_path = setup_test_db(&tmp, 5);
        let store = VectorStore::new(tmp.path().join("vectors.db"));
        let provider = Box::new(TestEmbeddingProvider::new());
        let pipeline = EmbeddingPipeline::new(provider, store);

        let mut last_progress = (0u64, 0u64);
        let count = index_all_commands(&db_path, &pipeline, |done, total| {
            last_progress = (done, total);
        })
        .unwrap();

        assert_eq!(count, 5);
        assert_eq!(last_progress, (5, 5));
        assert_eq!(pipeline.vector_count().unwrap(), 5);
    }

    #[test]
    fn test_incremental_indexing() {
        let tmp = TempDir::new().unwrap();
        let db_path = setup_test_db(&tmp, 3);
        let store = VectorStore::new(tmp.path().join("vectors.db"));
        let provider = Box::new(TestEmbeddingProvider::new());
        let pipeline = EmbeddingPipeline::new(provider, store);

        // First run: index all 3.
        let count1 = index_all_commands(&db_path, &pipeline, |_, _| {}).unwrap();
        assert_eq!(count1, 3);

        // Second run: nothing new to index.
        let count2 = index_all_commands(&db_path, &pipeline, |_, _| {}).unwrap();
        assert_eq!(count2, 0);
    }

    #[test]
    fn test_reindex_clears_and_rebuilds() {
        let tmp = TempDir::new().unwrap();
        let db_path = setup_test_db(&tmp, 4);
        let store = VectorStore::new(tmp.path().join("vectors.db"));
        let provider = Box::new(TestEmbeddingProvider::new());
        let pipeline = EmbeddingPipeline::new(provider, store);

        // Index all.
        index_all_commands(&db_path, &pipeline, |_, _| {}).unwrap();
        assert_eq!(pipeline.vector_count().unwrap(), 4);

        // Reindex: should clear and rebuild.
        let count = reindex_all_commands(&db_path, &pipeline, |_, _| {}).unwrap();
        assert_eq!(count, 4);
        assert_eq!(pipeline.vector_count().unwrap(), 4);
    }
}
