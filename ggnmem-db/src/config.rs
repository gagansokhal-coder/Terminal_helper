use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub runtime: RuntimeConfig,
    pub model: ModelConfig,
}

impl AppConfig {
    #[must_use]
    pub fn new(database_path: PathBuf) -> Self {
        Self {
            database: DatabaseConfig::new(database_path),
            runtime: RuntimeConfig::default(),
            model: ModelConfig::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: PathBuf,
    pub busy_timeout_ms: u64,
    pub mmap_size_bytes: u64,
}

impl DatabaseConfig {
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            busy_timeout_ms: 2_000,
            mmap_size_bytes: 30_000_000_000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub cli_timeout_ms: u64,
    pub daemon_idle_memory_limit_mb: u64,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            cli_timeout_ms: 10,
            daemon_idle_memory_limit_mb: 50,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelConfig {
    pub embedding_dimensions: u16,
    pub model_name: String,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            embedding_dimensions: 384,
            model_name: "all-MiniLM-L6-v2".to_owned(),
        }
    }
}
