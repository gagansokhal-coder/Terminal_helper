//! AI-specific configuration for ggnmem.
//!
//! Separate from the main CLI config to keep AI concerns isolated.
//! The CLI config module maps its `[ai]` section into these types.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// AI subsystem configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    /// Whether AI features are enabled globally.
    pub enabled: bool,

    /// Embedding provider to use. Currently only "local" is supported.
    pub embedding_provider: String,

    /// Whether semantic search is active (requires enabled + model installed).
    pub semantic_search: bool,

    /// Name of the embedding model to use.
    pub model_name: String,

    /// Directory where model files are stored.
    /// Separate from the core database directory.
    pub models_dir: PathBuf,

    /// Path to the vector database file.
    /// Separate from the core ggnmem.db.
    pub vector_db_path: PathBuf,
}

impl Default for AiConfig {
    fn default() -> Self {
        // Default paths use ~/.local/share/ggnmem/ai/ to keep AI assets
        // separate from the core database.
        let base = default_ai_data_dir();
        Self {
            enabled: false,
            embedding_provider: "local".to_owned(),
            semantic_search: false,
            model_name: "all-MiniLM-L6-v2".to_owned(),
            models_dir: base.join("models"),
            vector_db_path: base.join("vectors.db"),
        }
    }
}

/// Get the default AI data directory.
///
/// Windows: `%LOCALAPPDATA%\ggnmem\ai\`
/// Unix:    `~/.local/share/ggnmem/ai/` (or `$XDG_DATA_HOME/ggnmem/ai/`)
fn default_ai_data_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            return PathBuf::from(local_app_data).join("ggnmem").join("ai");
        }
    }

    #[cfg(unix)]
    {
        if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
            return PathBuf::from(data_home).join("ggnmem").join("ai");
        }

        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("ggnmem")
                .join("ai");
        }
    }

    // Fallback for environments without HOME / LOCALAPPDATA.
    PathBuf::from("ggnmem-ai-data")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_disabled() {
        let cfg = AiConfig::default();
        assert!(!cfg.enabled);
        assert!(!cfg.semantic_search);
        assert_eq!(cfg.embedding_provider, "local");
        assert_eq!(cfg.model_name, "all-MiniLM-L6-v2");
    }
}
