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

// ─── N-gram Embedding Provider ───────────────────────────────────────────────

/// A lightweight embedding provider that uses character n-grams and
/// word-level features to produce semantically useful similarity.
///
/// This is NOT a neural model — it uses deterministic feature hashing
/// to project text into a fixed-size vector space. It handles:
/// - Typo tolerance ("dockr" ≈ "docker" via shared trigrams)
/// - Word overlap ("check git changes" ≈ "git status" via shared "git")
/// - Substring similarity ("git log" ≈ "git status" via "git" prefix)
///
/// For production semantic search (understanding synonyms, paraphrasing),
/// a real neural model (ONNX/Candle) is needed in a future phase.
pub struct NgramEmbeddingProvider {
    dimensions: usize,
    model_name: String,
}

impl NgramEmbeddingProvider {
    /// Create a provider with default settings (384 dimensions).
    #[must_use]
    pub fn new() -> Self {
        Self {
            dimensions: EMBEDDING_DIMENSIONS,
            model_name: "all-MiniLM-L6-v2".to_owned(),
        }
    }

    /// Create a provider with custom dimensions.
    #[must_use]
    pub fn with_dimensions(dimensions: usize) -> Self {
        Self {
            dimensions,
            model_name: "all-MiniLM-L6-v2".to_owned(),
        }
    }
}

impl Default for NgramEmbeddingProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddingProvider for NgramEmbeddingProvider {
    fn embed_query(&self, query: &str) -> Result<Vec<f32>, AiError> {
        Ok(ngram_embedding(query, self.dimensions))
    }

    fn embed_command(&self, command: &str) -> Result<Vec<f32>, AiError> {
        Ok(ngram_embedding(command, self.dimensions))
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

/// Backward-compatible alias for tests and existing call sites.
pub type TestEmbeddingProvider = NgramEmbeddingProvider;

/// Generate an embedding from text using character n-grams and word features.
///
/// The embedding encodes:
/// 1. **Whole words** (weight 2.0) — strongest signal for exact word overlap.
/// 2. **Character trigrams** (weight 1.0) — handles typos and substrings.
/// 3. **Character bigrams** (weight 0.5) — additional typo tolerance.
///
/// Feature hashing maps each feature to a fixed-size vector position.
/// The vector is L2-normalized for cosine similarity compatibility.
fn ngram_embedding(text: &str, dimensions: usize) -> Vec<f32> {
    let mut vec = vec![0.0f32; dimensions];
    let text_lower = text.to_lowercase();

    for word in text_lower.split_whitespace() {
        // Whole-word features (strongest signal).
        let idx = stable_hash(word.as_bytes()) % dimensions;
        vec[idx] += 2.0;

        // Character trigrams within padded word.
        let padded = format!(" {word} ");
        for window in padded.as_bytes().windows(3) {
            let idx = stable_hash(window) % dimensions;
            vec[idx] += 1.0;
        }

        // Character bigrams for additional overlap.
        if word.len() >= 2 {
            for window in word.as_bytes().windows(2) {
                let idx = stable_hash(window) % dimensions;
                vec[idx] += 0.5;
            }
        }
    }

    // L2 normalize to unit length.
    let magnitude: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if magnitude > 0.0 {
        for v in &mut vec {
            *v /= magnitude;
        }
    }

    vec
}

/// Deterministic hash for feature hashing (FNV-1a variant).
/// NOT cryptographic — designed for speed and uniform distribution.
fn stable_hash(bytes: &[u8]) -> usize {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325; // FNV offset basis
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x0100_0000_01b3); // FNV prime
    }
    hash as usize
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

        assert_eq!(pipeline.model_name(), "all-MiniLM-L6-v2");
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
