//! Local embedding model management.
//!
//! Manages a registry of supported local embedding models and tracks
//! their install state on disk. Models are stored in a dedicated
//! directory separate from the core database.
//!
//! Phase 12A: plumbing only — no actual model downloads.
//! `install()` creates the directory structure with a marker file.
//! A future phase will add actual model file downloading/extraction.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{AiError, AiResult};

/// Metadata about an embedding model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model identifier (e.g., "all-MiniLM-L6-v2").
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Expected model file size in bytes (approximate).
    pub size_bytes: u64,
    /// Embedding output dimensions.
    pub dimensions: usize,
    /// Whether the model is currently installed.
    pub installed: bool,
    /// Disk path if installed.
    pub install_path: Option<PathBuf>,
    /// Actual size on disk (0 if not installed).
    pub disk_size_bytes: u64,
}

/// A registry entry for a supported model (compile-time constant).
struct ModelRegistryEntry {
    name: &'static str,
    description: &'static str,
    size_bytes: u64,
    dimensions: usize,
}

/// Supported local embedding models.
/// These are the models that ggnmem can manage.
/// No downloads happen in Phase 12A — this is metadata only.
const MODEL_REGISTRY: &[ModelRegistryEntry] = &[
    ModelRegistryEntry {
        name: "all-MiniLM-L6-v2",
        description: "Sentence-Transformers all-MiniLM-L6-v2 (ONNX, ~80 MB)",
        size_bytes: 80_000_000,
        dimensions: 384,
    },
    ModelRegistryEntry {
        name: "bge-small-en-v1.5",
        description: "BAAI bge-small-en-v1.5 (ONNX, ~130 MB)",
        size_bytes: 130_000_000,
        dimensions: 384,
    },
];

const MODEL_MARKER_FILE: &str = ".ggnmem-model";

/// Manages local embedding model installation.
pub struct ModelManager {
    models_dir: PathBuf,
}

impl ModelManager {
    /// Create a model manager pointing at the given directory.
    /// Does NOT perform any I/O — the directory is created lazily on install.
    #[must_use]
    pub fn new(models_dir: PathBuf) -> Self {
        Self { models_dir }
    }

    /// List all models from the registry with their install status.
    pub fn list_available(&self) -> Vec<ModelInfo> {
        MODEL_REGISTRY
            .iter()
            .map(|entry| {
                let model_dir = self.models_dir.join(entry.name);
                let installed = is_model_dir_valid(&model_dir);
                let disk_size = if installed {
                    dir_size(&model_dir).unwrap_or(0)
                } else {
                    0
                };
                ModelInfo {
                    name: entry.name.to_owned(),
                    description: entry.description.to_owned(),
                    size_bytes: entry.size_bytes,
                    dimensions: entry.dimensions,
                    installed,
                    install_path: if installed { Some(model_dir) } else { None },
                    disk_size_bytes: disk_size,
                }
            })
            .collect()
    }

    /// List only installed models.
    pub fn list_installed(&self) -> Vec<ModelInfo> {
        self.list_available()
            .into_iter()
            .filter(|m| m.installed)
            .collect()
    }

    /// Check if a specific model is installed.
    #[must_use]
    pub fn is_installed(&self, name: &str) -> bool {
        let model_dir = self.models_dir.join(name);
        is_model_dir_valid(&model_dir)
    }

    /// Get info about a specific model (must be in registry).
    pub fn get_model(&self, name: &str) -> AiResult<ModelInfo> {
        self.list_available()
            .into_iter()
            .find(|m| m.name == name)
            .ok_or_else(|| AiError::UnknownModel(name.to_owned()))
    }

    /// Get the disk size of an installed model in bytes.
    pub fn model_size(&self, name: &str) -> Option<u64> {
        let model_dir = self.models_dir.join(name);
        if is_model_dir_valid(&model_dir) {
            dir_size(&model_dir).ok()
        } else {
            None
        }
    }

    /// "Install" a model by creating its directory structure.
    ///
    /// Phase 12A: creates the directory + marker file only.
    /// A future phase will download actual model weights here.
    pub fn install(&self, name: &str) -> AiResult<ModelInfo> {
        // Validate the model exists in registry.
        let entry = MODEL_REGISTRY
            .iter()
            .find(|e| e.name == name)
            .ok_or_else(|| AiError::UnknownModel(name.to_owned()))?;

        let model_dir = self.models_dir.join(name);

        if is_model_dir_valid(&model_dir) {
            return Err(AiError::ModelAlreadyInstalled(name.to_owned()));
        }

        // Create model directory + marker.
        fs::create_dir_all(&model_dir)?;

        let marker_content = format!(
            "name={}\ndimensions={}\nsize_bytes={}\nstatus=placeholder\n",
            entry.name, entry.dimensions, entry.size_bytes
        );
        fs::write(model_dir.join(MODEL_MARKER_FILE), marker_content)?;

        Ok(ModelInfo {
            name: entry.name.to_owned(),
            description: entry.description.to_owned(),
            size_bytes: entry.size_bytes,
            dimensions: entry.dimensions,
            installed: true,
            install_path: Some(model_dir),
            disk_size_bytes: 0, // Placeholder, no real weights yet.
        })
    }

    /// Remove an installed model by deleting its directory.
    pub fn remove(&self, name: &str) -> AiResult<()> {
        // Validate the model exists in registry.
        if !MODEL_REGISTRY.iter().any(|e| e.name == name) {
            return Err(AiError::UnknownModel(name.to_owned()));
        }

        let model_dir = self.models_dir.join(name);

        if !is_model_dir_valid(&model_dir) {
            return Err(AiError::ModelNotInstalled(name.to_owned()));
        }

        fs::remove_dir_all(&model_dir)?;
        Ok(())
    }
}

/// Check if a model directory is valid (has the marker file).
fn is_model_dir_valid(model_dir: &Path) -> bool {
    model_dir.join(MODEL_MARKER_FILE).exists()
}

/// Calculate the total size of a directory recursively.
fn dir_size(path: &Path) -> std::io::Result<u64> {
    let mut total: u64 = 0;
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let meta = entry.metadata()?;
            if meta.is_dir() {
                total += dir_size(&entry.path())?;
            } else {
                total += meta.len();
            }
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn list_available_returns_registry() {
        let tmp = TempDir::new().unwrap();
        let mgr = ModelManager::new(tmp.path().to_path_buf());
        let models = mgr.list_available();

        assert_eq!(models.len(), 2);
        assert_eq!(models[0].name, "all-MiniLM-L6-v2");
        assert_eq!(models[1].name, "bge-small-en-v1.5");
        assert!(!models[0].installed);
        assert!(!models[1].installed);
    }

    #[test]
    fn install_creates_directory_and_marker() {
        let tmp = TempDir::new().unwrap();
        let mgr = ModelManager::new(tmp.path().to_path_buf());

        let info = mgr.install("all-MiniLM-L6-v2").unwrap();
        assert!(info.installed);
        assert!(info.install_path.is_some());
        assert!(mgr.is_installed("all-MiniLM-L6-v2"));

        let marker = tmp.path().join("all-MiniLM-L6-v2").join(MODEL_MARKER_FILE);
        assert!(marker.exists());
    }

    #[test]
    fn install_duplicate_returns_error() {
        let tmp = TempDir::new().unwrap();
        let mgr = ModelManager::new(tmp.path().to_path_buf());

        mgr.install("all-MiniLM-L6-v2").unwrap();
        let result = mgr.install("all-MiniLM-L6-v2");
        assert!(result.is_err());
    }

    #[test]
    fn install_unknown_model_returns_error() {
        let tmp = TempDir::new().unwrap();
        let mgr = ModelManager::new(tmp.path().to_path_buf());

        let result = mgr.install("nonexistent-model");
        assert!(result.is_err());
    }

    #[test]
    fn remove_deletes_directory() {
        let tmp = TempDir::new().unwrap();
        let mgr = ModelManager::new(tmp.path().to_path_buf());

        mgr.install("all-MiniLM-L6-v2").unwrap();
        assert!(mgr.is_installed("all-MiniLM-L6-v2"));

        mgr.remove("all-MiniLM-L6-v2").unwrap();
        assert!(!mgr.is_installed("all-MiniLM-L6-v2"));
    }

    #[test]
    fn remove_uninstalled_returns_error() {
        let tmp = TempDir::new().unwrap();
        let mgr = ModelManager::new(tmp.path().to_path_buf());

        let result = mgr.remove("all-MiniLM-L6-v2");
        assert!(result.is_err());
    }

    #[test]
    fn list_installed_only_returns_installed() {
        let tmp = TempDir::new().unwrap();
        let mgr = ModelManager::new(tmp.path().to_path_buf());

        assert!(mgr.list_installed().is_empty());

        mgr.install("bge-small-en-v1.5").unwrap();
        let installed = mgr.list_installed();
        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0].name, "bge-small-en-v1.5");
    }

    #[test]
    fn model_size_returns_none_when_not_installed() {
        let tmp = TempDir::new().unwrap();
        let mgr = ModelManager::new(tmp.path().to_path_buf());
        assert!(mgr.model_size("all-MiniLM-L6-v2").is_none());
    }

    #[test]
    fn model_size_returns_size_when_installed() {
        let tmp = TempDir::new().unwrap();
        let mgr = ModelManager::new(tmp.path().to_path_buf());

        mgr.install("all-MiniLM-L6-v2").unwrap();
        let size = mgr.model_size("all-MiniLM-L6-v2");
        // Marker file is the only content, so size should be small but nonzero.
        assert!(size.is_some());
        assert!(size.unwrap() > 0);
    }
}
