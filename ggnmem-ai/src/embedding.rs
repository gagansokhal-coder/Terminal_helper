//! Embedding pipeline interfaces and test implementations.
//!
//! Defines the `EmbeddingProvider` trait for generating vector embeddings
//! from text, and the `EmbeddingPipeline` orchestrator that combines a
//! provider with a vector store.
//!
//! Phase 12A: interfaces only + test implementation.
//! No real model loading, no ML runtime, no inference.
//! The `TestEmbeddingProvider` generates deterministic pseudo-embeddings
//! based on string hashing for test/development use.

use sha2::{Digest, Sha256};

use crate::error::{AiError, AiResult};
use crate::vector::{VectorMatch, VectorStore, EMBEDDING_DIMENSIONS};

// ─── Embedding Provider Trait ────────────────────────────────────────────────

/// Trait for generating vector embeddings from text.
///
/// Implementations of this trait produce fixed-size float vectors
/// from text input. Different implementations can use different
/// models or inference backends.
///
/// Phase 12A provides only the `TestEmbeddingProvider` implementation.
/// Future phases will add real model-backed providers.
pub trait EmbeddingProvider: Send + Sync {
    /// Generate an embedding vector for a search query.
    ///
    /// Query embeddings may be generated differently from document
    /// embeddings in some model architectures (e.g., asymmetric search).
    fn embed_query(&self, query: &str) -> Result<Vec<f32>, AiError>;

    /// Generate an embedding vector for a command string.
    ///
    /// This is the "document" embedding used for indexing.
    fn embed_command(&self, command: &str) -> Result<Vec<f32>, AiError>;

    /// The number of dimensions in the output embeddings.
    fn dimensions(&self) -> usize;

    /// The name of the model being used.
    fn model_name(&self) -> &str;
}

// ─── Test Embedding Provider ─────────────────────────────────────────────────

/// A deterministic test implementation of `EmbeddingProvider`.
///
/// Generates pseudo-embeddings by hashing the input text with SHA-256
/// and distributing the hash bytes across the embedding dimensions.
/// This produces consistent, reproducible embeddings for testing
/// without any ML dependencies.
///
/// The embeddings are NOT semantically meaningful — "docker" and
/// "container" will NOT be close in vector space. This is purely
/// for testing the pipeline plumbing.
pub struct TestEmbeddingProvider {
    dimensions: usize,
    model_name: String,
}

impl TestEmbeddingProvider {
    /// Create a test provider with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            dimensions: EMBEDDING_DIMENSIONS,
            model_name: "test-embedding-provider".to_owned(),
        }
    }

    /// Create a test provider with custom dimensions.
    #[must_use]
    pub fn with_dimensions(dimensions: usize) -> Self {
        Self {
            dimensions,
            model_name: "test-embedding-provider".to_owned(),
        }
    }
}

impl Default for TestEmbeddingProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddingProvider for TestEmbeddingProvider {
    fn embed_query(&self, query: &str) -> Result<Vec<f32>, AiError> {
        Ok(hash_to_embedding(query, self.dimensions))
    }

    fn embed_command(&self, command: &str) -> Result<Vec<f32>, AiError> {
        Ok(hash_to_embedding(command, self.dimensions))
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

/// Generate a deterministic pseudo-embedding from a string.
///
/// Uses SHA-256 to hash the input, then maps the hash bytes to f32 values
/// in the range [-1.0, 1.0]. The hash is extended by re-hashing with
/// incrementing prefixes to fill all dimensions.
fn hash_to_embedding(text: &str, dimensions: usize) -> Vec<f32> {
    let mut embedding = Vec::with_capacity(dimensions);
    let mut round = 0u32;

    while embedding.len() < dimensions {
        let mut hasher = Sha256::new();
        hasher.update(round.to_le_bytes());
        hasher.update(text.as_bytes());
        let hash = hasher.finalize();

        // Each hash byte maps to one f32 in [-1.0, 1.0].
        for byte in hash.iter() {
            if embedding.len() >= dimensions {
                break;
            }
            // Map 0..255 to -1.0..1.0.
            let value = (*byte as f32 / 127.5) - 1.0;
            embedding.push(value);
        }

        round += 1;
    }

    // Normalize to unit length for cosine similarity compatibility.
    let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if magnitude > 0.0 {
        for v in &mut embedding {
            *v /= magnitude;
        }
    }

    embedding
}

// ─── Embedding Pipeline ──────────────────────────────────────────────────────

/// Orchestrator that combines an embedding provider with a vector store.
///
/// The pipeline handles:
/// - Generating embeddings from text via the provider
/// - Storing embeddings in the vector store
/// - Searching for similar vectors
///
/// Phase 12A: the pipeline is fully functional with the test provider.
/// Real semantic search requires a model-backed provider (future phase).
pub struct EmbeddingPipeline {
    provider: Box<dyn EmbeddingProvider>,
    store: VectorStore,
}

impl EmbeddingPipeline {
    /// Create a new embedding pipeline.
    #[must_use]
    pub fn new(provider: Box<dyn EmbeddingProvider>, store: VectorStore) -> Self {
        Self { provider, store }
    }

    /// Generate an embedding for a command and store it.
    pub fn index_embedding(&self, command_id: &str, command: &str) -> AiResult<()> {
        let embedding = self.provider.embed_command(command)?;
        self.store.insert(command_id, &embedding)?;
        Ok(())
    }

    /// Search for commands similar to a query string.
    pub fn search_embedding(&self, query: &str, limit: usize) -> AiResult<Vec<VectorMatch>> {
        let query_embedding = self.provider.embed_query(query)?;
        self.store.search(&query_embedding, limit)
    }

    /// Get the number of indexed vectors.
    pub fn vector_count(&self) -> AiResult<u64> {
        self.store.count()
    }

    /// Get the model name being used.
    pub fn model_name(&self) -> &str {
        self.provider.model_name()
    }

    /// Get the embedding dimensions.
    pub fn dimensions(&self) -> usize {
        self.provider.dimensions()
    }

    /// Index a batch of commands, calling `progress` after each one.
    ///
    /// `commands` is a slice of `(command_id, command_text)` pairs.
    /// The callback receives `(completed_count, total_count)`.
    pub fn batch_index(
        &self,
        commands: &[(String, String)],
        mut progress: impl FnMut(u64, u64),
    ) -> AiResult<u64> {
        let total = commands.len() as u64;
        let mut indexed = 0u64;

        for (id, text) in commands {
            self.index_embedding(id, text)?;
            indexed += 1;
            progress(indexed, total);
        }

        Ok(indexed)
    }

    /// Delete all embeddings from the vector store (for reindexing).
    pub fn delete_all_embeddings(&self) -> AiResult<u64> {
        self.store.delete_all()
    }

    /// Get the set of already-indexed command IDs.
    pub fn indexed_ids(&self) -> AiResult<std::collections::HashSet<String>> {
        self.store.list_indexed_ids()
    }

    /// Access the underlying embedding provider.
    pub fn provider(&self) -> &dyn EmbeddingProvider {
        self.provider.as_ref()
    }
}

// ─── Standalone interface functions ──────────────────────────────────────────
// These match the Phase 12A spec for interface stubs.

/// Generate an embedding for a search query.
///
/// Convenience wrapper — in production use the `EmbeddingPipeline` instead.
pub fn embed_query(provider: &dyn EmbeddingProvider, query: &str) -> AiResult<Vec<f32>> {
    provider.embed_query(query)
}

/// Generate an embedding for a command string.
///
/// Convenience wrapper — in production use the `EmbeddingPipeline` instead.
pub fn embed_command(provider: &dyn EmbeddingProvider, command: &str) -> AiResult<Vec<f32>> {
    provider.embed_command(command)
}

/// Index an embedding in a vector store.
///
/// Convenience wrapper — in production use the `EmbeddingPipeline` instead.
pub fn index_embedding(store: &VectorStore, id: &str, embedding: &[f32]) -> AiResult<()> {
    store.insert(id, embedding)
}

/// Search for similar embeddings in a vector store.
///
/// Convenience wrapper — in production use the `EmbeddingPipeline` instead.
pub fn search_embedding(
    store: &VectorStore,
    query_embedding: &[f32],
    limit: usize,
) -> AiResult<Vec<VectorMatch>> {
    store.search(query_embedding, limit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_embed_query_returns_correct_dimensions() {
        let provider = TestEmbeddingProvider::new();
        let embedding = provider.embed_query("docker compose up").unwrap();
        assert_eq!(embedding.len(), EMBEDDING_DIMENSIONS);
    }

    #[test]
    fn test_embed_command_returns_correct_dimensions() {
        let provider = TestEmbeddingProvider::new();
        let embedding = provider.embed_command("git push origin main").unwrap();
        assert_eq!(embedding.len(), EMBEDDING_DIMENSIONS);
    }

    #[test]
    fn test_embeddings_are_deterministic() {
        let provider = TestEmbeddingProvider::new();
        let e1 = provider.embed_query("hello").unwrap();
        let e2 = provider.embed_query("hello").unwrap();
        assert_eq!(e1, e2);
    }

    #[test]
    fn test_different_inputs_produce_different_embeddings() {
        let provider = TestEmbeddingProvider::new();
        let e1 = provider.embed_query("docker").unwrap();
        let e2 = provider.embed_query("kubernetes").unwrap();
        assert_ne!(e1, e2);
    }

    #[test]
    fn test_embeddings_are_normalized() {
        let provider = TestEmbeddingProvider::new();
        let embedding = provider.embed_query("test normalization").unwrap();
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        // Should be approximately 1.0 (unit vector).
        assert!((magnitude - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_pipeline_index_and_count() {
        let tmp = TempDir::new().unwrap();
        let store = VectorStore::new(tmp.path().join("vectors.db"));
        let provider = Box::new(TestEmbeddingProvider::new());
        let pipeline = EmbeddingPipeline::new(provider, store);

        pipeline
            .index_embedding("cmd-1", "docker compose up")
            .unwrap();
        pipeline
            .index_embedding("cmd-2", "git push origin main")
            .unwrap();
        pipeline
            .index_embedding("cmd-3", "cargo test --workspace")
            .unwrap();

        assert_eq!(pipeline.vector_count().unwrap(), 3);
    }

    #[test]
    fn test_pipeline_search_returns_results() {
        let tmp = TempDir::new().unwrap();
        let store = VectorStore::new(tmp.path().join("vectors.db"));
        let provider = Box::new(TestEmbeddingProvider::new());
        let pipeline = EmbeddingPipeline::new(provider, store);

        pipeline
            .index_embedding("cmd-1", "docker compose up")
            .unwrap();

        // Brute-force fallback now returns results.
        let results = pipeline.search_embedding("docker compose up", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "cmd-1");
    }

    #[test]
    fn test_pipeline_batch_index() {
        let tmp = TempDir::new().unwrap();
        let store = VectorStore::new(tmp.path().join("vectors.db"));
        let provider = Box::new(TestEmbeddingProvider::new());
        let pipeline = EmbeddingPipeline::new(provider, store);

        let commands = vec![
            ("cmd-1".to_owned(), "docker compose up".to_owned()),
            ("cmd-2".to_owned(), "git push origin main".to_owned()),
            ("cmd-3".to_owned(), "cargo test --workspace".to_owned()),
        ];

        let mut last_progress = (0u64, 0u64);
        let count = pipeline
            .batch_index(&commands, |done, total| {
                last_progress = (done, total);
            })
            .unwrap();

        assert_eq!(count, 3);
        assert_eq!(last_progress, (3, 3));
        assert_eq!(pipeline.vector_count().unwrap(), 3);
    }

    #[test]
    fn test_pipeline_delete_all() {
        let tmp = TempDir::new().unwrap();
        let store = VectorStore::new(tmp.path().join("vectors.db"));
        let provider = Box::new(TestEmbeddingProvider::new());
        let pipeline = EmbeddingPipeline::new(provider, store);

        pipeline
            .index_embedding("cmd-1", "docker compose up")
            .unwrap();
        pipeline.index_embedding("cmd-2", "git push").unwrap();
        assert_eq!(pipeline.vector_count().unwrap(), 2);

        let removed = pipeline.delete_all_embeddings().unwrap();
        assert_eq!(removed, 2);
        assert_eq!(pipeline.vector_count().unwrap(), 0);
    }

    #[test]
    fn test_pipeline_model_info() {
        let tmp = TempDir::new().unwrap();
        let store = VectorStore::new(tmp.path().join("vectors.db"));
        let provider = Box::new(TestEmbeddingProvider::new());
        let pipeline = EmbeddingPipeline::new(provider, store);

        assert_eq!(pipeline.model_name(), "test-embedding-provider");
        assert_eq!(pipeline.dimensions(), EMBEDDING_DIMENSIONS);
    }

    #[test]
    fn test_standalone_embed_query() {
        let provider = TestEmbeddingProvider::new();
        let result = embed_query(&provider, "test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), EMBEDDING_DIMENSIONS);
    }

    #[test]
    fn test_standalone_embed_command() {
        let provider = TestEmbeddingProvider::new();
        let result = embed_command(&provider, "ls -la");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), EMBEDDING_DIMENSIONS);
    }

    #[test]
    fn test_standalone_index_embedding() {
        let tmp = TempDir::new().unwrap();
        let store = VectorStore::new(tmp.path().join("vectors.db"));
        let embedding = vec![0.0_f32; EMBEDDING_DIMENSIONS];
        index_embedding(&store, "cmd-1", &embedding).unwrap();
        assert_eq!(store.count().unwrap(), 1);
    }

    #[test]
    fn test_standalone_search_embedding() {
        let tmp = TempDir::new().unwrap();
        let store = VectorStore::new(tmp.path().join("vectors.db"));
        let mut embedding = vec![0.0_f32; EMBEDDING_DIMENSIONS];
        embedding[0] = 1.0; // non-zero unit vector
        index_embedding(&store, "cmd-1", &embedding).unwrap();
        let results = search_embedding(&store, &embedding, 10).unwrap();
        // Brute-force fallback returns results.
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "cmd-1");
    }

    #[test]
    fn test_custom_dimensions_provider() {
        let provider = TestEmbeddingProvider::with_dimensions(128);
        assert_eq!(provider.dimensions(), 128);
        let embedding = provider.embed_query("test").unwrap();
        assert_eq!(embedding.len(), 128);
    }
}
