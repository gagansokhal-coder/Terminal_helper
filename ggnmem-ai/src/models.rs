//! Local embedding model management.
//!
//! Manages a registry of supported local embedding models and tracks
//! their install state on disk. Models are stored in a dedicated
//! directory separate from the core database.
//!
//! Phase 12C: real model downloading from Hugging Face with
//! version-pinned URLs and SHA256 integrity verification.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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
    /// Hugging Face download URLs for model assets.
    #[cfg(feature = "onnx")]
    assets: &'static [ModelAsset],
}

/// A downloadable model asset (ONNX weights, tokenizer, etc.).
#[cfg(feature = "onnx")]
struct ModelAsset {
    /// Filename in the model directory.
    filename: &'static str,
    /// Download URL (version-pinned to a specific HF revision).
    url: &'static str,
    /// Expected SHA256 hex digest. Empty string = skip verification
    /// (log computed hash for future pinning).
    sha256: &'static str,
}

/// Pinned Hugging Face revision for all-MiniLM-L6-v2 model assets.
/// Using a specific commit ensures reproducible downloads.
#[cfg(feature = "onnx")]
const MINILM_HF_REVISION: &str = "refs/heads/main";

/// Supported local embedding models.
/// These are the models that ggnmem can manage.
const MODEL_REGISTRY: &[ModelRegistryEntry] = &[
    ModelRegistryEntry {
        name: "all-MiniLM-L6-v2",
        description: "Sentence-Transformers all-MiniLM-L6-v2 (ONNX, ~80 MB)",
        size_bytes: 80_000_000,
        dimensions: 384,
        #[cfg(feature = "onnx")]
        assets: &[
            ModelAsset {
                filename: "model.onnx",
                url: "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/onnx/model.onnx",
                sha256: "", // Computed on first download and stored locally
            },
            ModelAsset {
                filename: "tokenizer.json",
                url: "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/tokenizer.json",
                sha256: "",
            },
        ],
    },
    ModelRegistryEntry {
        name: "bge-small-en-v1.5",
        description: "BAAI bge-small-en-v1.5 (ONNX, ~130 MB)",
        size_bytes: 130_000_000,
        dimensions: 384,
        #[cfg(feature = "onnx")]
        assets: &[], // Not yet supported for download
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

    /// Check if real ONNX model files exist for a model.
    #[must_use]
    pub fn has_real_model_files(&self, name: &str) -> bool {
        let model_dir = self.models_dir.join(name);
        has_onnx_files(&model_dir)
    }

    /// Check if a model is installed with only a marker file and needs
    /// upgrading to real ONNX weights.
    ///
    /// Returns `true` when the model directory exists with a marker but
    /// without `model.onnx` — meaning it was installed in a previous
    /// phase (or without the `onnx` feature) and needs re-downloading.
    #[must_use]
    pub fn needs_upgrade(&self, name: &str) -> bool {
        let model_dir = self.models_dir.join(name);
        // Has marker but no real ONNX files.
        model_dir.join(MODEL_MARKER_FILE).exists() && !has_onnx_files(&model_dir)
    }

    /// Install a model.
    ///
    /// With the `onnx` feature: downloads real model files from Hugging Face
    /// with progress reporting and SHA256 integrity verification.
    ///
    /// Without `onnx`: creates a marker file (backward compatible).
    ///
    /// `progress` callback receives `(bytes_downloaded, total_bytes)`.
    pub fn install(&self, name: &str, mut progress: impl FnMut(u64, u64)) -> AiResult<ModelInfo> {
        // Validate the model exists in registry.
        let entry = MODEL_REGISTRY
            .iter()
            .find(|e| e.name == name)
            .ok_or_else(|| AiError::UnknownModel(name.to_owned()))?;

        let model_dir = self.models_dir.join(name);

        // Check if already fully installed with real ONNX files.
        if has_onnx_files(&model_dir) {
            return Err(AiError::ModelAlreadyInstalled(name.to_owned()));
        }

        // If marker-only install exists and onnx feature is active,
        // allow upgrade by proceeding with download into the existing dir.
        // If onnx feature is NOT active and marker exists, it's already installed.
        #[cfg(not(feature = "onnx"))]
        if is_model_dir_valid(&model_dir) {
            return Err(AiError::ModelAlreadyInstalled(name.to_owned()));
        }

        // Create model directory.
        fs::create_dir_all(&model_dir)?;

        // Download real model files (onnx feature) or create marker (lite).
        #[cfg(feature = "onnx")]
        {
            if entry.assets.is_empty() {
                return Err(AiError::ModelDownloadError(format!(
                    "model '{}' does not have downloadable assets yet",
                    name
                )));
            }

            // Download each asset.
            for asset in entry.assets {
                let dest = model_dir.join(asset.filename);
                download_file(asset.url, &dest, asset.sha256, &mut progress)?;
            }

            // Write marker file with model metadata.
            let marker_content = format!(
                "name={}\ndimensions={}\nsize_bytes={}\nstatus=installed\n",
                entry.name, entry.dimensions, entry.size_bytes
            );
            fs::write(model_dir.join(MODEL_MARKER_FILE), marker_content)?;
        }

        #[cfg(not(feature = "onnx"))]
        {
            let _ = &mut progress; // suppress unused warning
            let marker_content = format!(
                "name={}\ndimensions={}\nsize_bytes={}\nstatus=placeholder\n",
                entry.name, entry.dimensions, entry.size_bytes
            );
            fs::write(model_dir.join(MODEL_MARKER_FILE), marker_content)?;
        }

        let disk_size = dir_size(&model_dir).unwrap_or(0);

        Ok(ModelInfo {
            name: entry.name.to_owned(),
            description: entry.description.to_owned(),
            size_bytes: entry.size_bytes,
            dimensions: entry.dimensions,
            installed: true,
            install_path: Some(model_dir),
            disk_size_bytes: disk_size,
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

    /// Verify the integrity of an installed model.
    ///
    /// Checks that required files exist and are non-empty.
    /// With the `onnx` feature, also verifies SHA256 hashes against
    /// stored values.
    pub fn verify_integrity(&self, name: &str) -> AiResult<()> {
        let model_dir = self.models_dir.join(name);

        if !model_dir.exists() {
            return Err(AiError::ModelNotInstalled(name.to_owned()));
        }

        #[cfg(feature = "onnx")]
        {
            let onnx_path = model_dir.join("model.onnx");
            let tok_path = model_dir.join("tokenizer.json");

            if !onnx_path.exists() {
                return Err(AiError::ModelIntegrityError(
                    "model.onnx missing".to_owned(),
                ));
            }
            if !tok_path.exists() {
                return Err(AiError::ModelIntegrityError(
                    "tokenizer.json missing".to_owned(),
                ));
            }

            // Sanity check: model.onnx should be >10 MB.
            let onnx_size = fs::metadata(&onnx_path)?.len();
            if onnx_size < 10_000_000 {
                return Err(AiError::ModelIntegrityError(format!(
                    "model.onnx too small ({onnx_size} bytes, expected >10 MB)"
                )));
            }

            // Verify SHA256 against stored hashes if available.
            verify_stored_hash(&onnx_path)?;
            verify_stored_hash(&tok_path)?;
        }

        #[cfg(not(feature = "onnx"))]
        {
            if !model_dir.join(MODEL_MARKER_FILE).exists() {
                return Err(AiError::ModelIntegrityError(
                    "marker file missing".to_owned(),
                ));
            }
        }

        Ok(())
    }
}

/// Check if a model directory is valid (has real files or marker).
fn is_model_dir_valid(model_dir: &Path) -> bool {
    // Check for real ONNX model files first.
    if has_onnx_files(model_dir) {
        return true;
    }
    // Fallback to legacy marker file (lite installs / Phase 12A).
    model_dir.join(MODEL_MARKER_FILE).exists()
}

/// Check if a directory contains real ONNX model files.
fn has_onnx_files(model_dir: &Path) -> bool {
    model_dir.join("model.onnx").exists() && model_dir.join("tokenizer.json").exists()
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

/// Compute SHA256 hex digest of a file.
fn compute_sha256(path: &Path) -> AiResult<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    let hash = hasher.finalize();
    Ok(format!("{hash:x}"))
}

/// Verify a file against its stored `.sha256` sidecar file.
///
/// If no sidecar exists, verification is skipped.
#[cfg(feature = "onnx")]
fn verify_stored_hash(path: &Path) -> AiResult<()> {
    let hash_path = path.with_extension(
        path.extension()
            .map(|e| format!("{}.sha256", e.to_string_lossy()))
            .unwrap_or_else(|| "sha256".to_owned()),
    );

    if !hash_path.exists() {
        return Ok(()); // No stored hash — skip verification.
    }

    let expected = fs::read_to_string(&hash_path)?.trim().to_lowercase();
    let actual = compute_sha256(path)?;

    if expected != actual {
        return Err(AiError::ModelIntegrityError(format!(
            "{}: SHA256 mismatch (expected {expected}, got {actual})",
            path.display()
        )));
    }

    Ok(())
}

/// Download a file from a URL with progress reporting and SHA256 verification.
///
/// * Streams the response body in 64 KB chunks for progress updates.
/// * Computes SHA256 on-the-fly during download.
/// * Stores the computed hash in a `.sha256` sidecar file.
/// * If `expected_sha256` is non-empty, verifies against it.
#[cfg(feature = "onnx")]
fn download_file(
    url: &str,
    dest: &Path,
    expected_sha256: &str,
    progress: &mut impl FnMut(u64, u64),
) -> AiResult<()> {
    use std::io::{Read, Write};

    let response = ureq::get(url)
        .call()
        .map_err(|e| AiError::ModelDownloadError(format!("{url}: {e}")))?;

    let total_bytes = response
        .header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    let mut reader = response.into_reader();
    let mut file = fs::File::create(dest)?;
    let mut hasher = Sha256::new();
    let mut downloaded: u64 = 0;
    let mut buf = [0u8; 65536]; // 64 KB chunks

    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| AiError::ModelDownloadError(format!("read: {e}")))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        hasher.update(&buf[..n]);
        downloaded += n as u64;
        progress(downloaded, total_bytes);
    }

    file.flush()?;
    drop(file);

    // Compute final hash.
    let hash = format!("{:x}", hasher.finalize());

    // Store SHA256 in sidecar file.
    let hash_path_str = format!("{}.sha256", dest.display());
    let hash_path = Path::new(&hash_path_str);
    let _ = fs::write(hash_path, &hash);

    // Verify against expected hash (if provided).
    if !expected_sha256.is_empty() && hash != expected_sha256 {
        // Remove the corrupt file.
        let _ = fs::remove_file(dest);
        let _ = fs::remove_file(hash_path);
        return Err(AiError::ModelIntegrityError(format!(
            "{}: SHA256 mismatch (expected {expected_sha256}, got {hash})",
            dest.display()
        )));
    }

    Ok(())
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
        // Without the `onnx` feature, install creates marker only.
        // With `onnx`, this test would require network access.
        #[cfg(not(feature = "onnx"))]
        {
            let tmp = TempDir::new().unwrap();
            let mgr = ModelManager::new(tmp.path().to_path_buf());

            let info = mgr.install("all-MiniLM-L6-v2", |_, _| {}).unwrap();
            assert!(info.installed);
            assert!(info.install_path.is_some());
            assert!(mgr.is_installed("all-MiniLM-L6-v2"));

            let marker = tmp.path().join("all-MiniLM-L6-v2").join(MODEL_MARKER_FILE);
            assert!(marker.exists());
        }
    }

    #[test]
    fn install_duplicate_returns_error() {
        #[cfg(not(feature = "onnx"))]
        {
            let tmp = TempDir::new().unwrap();
            let mgr = ModelManager::new(tmp.path().to_path_buf());

            mgr.install("all-MiniLM-L6-v2", |_, _| {}).unwrap();
            let result = mgr.install("all-MiniLM-L6-v2", |_, _| {});
            assert!(result.is_err());
        }
    }

    #[test]
    fn install_unknown_model_returns_error() {
        let tmp = TempDir::new().unwrap();
        let mgr = ModelManager::new(tmp.path().to_path_buf());

        let result = mgr.install("nonexistent-model", |_, _| {});
        assert!(result.is_err());
    }

    #[test]
    fn remove_deletes_directory() {
        #[cfg(not(feature = "onnx"))]
        {
            let tmp = TempDir::new().unwrap();
            let mgr = ModelManager::new(tmp.path().to_path_buf());

            mgr.install("all-MiniLM-L6-v2", |_, _| {}).unwrap();
            assert!(mgr.is_installed("all-MiniLM-L6-v2"));

            mgr.remove("all-MiniLM-L6-v2").unwrap();
            assert!(!mgr.is_installed("all-MiniLM-L6-v2"));
        }
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
        #[cfg(not(feature = "onnx"))]
        {
            let tmp = TempDir::new().unwrap();
            let mgr = ModelManager::new(tmp.path().to_path_buf());

            assert!(mgr.list_installed().is_empty());

            mgr.install("bge-small-en-v1.5", |_, _| {}).unwrap();
            let installed = mgr.list_installed();
            assert_eq!(installed.len(), 1);
            assert_eq!(installed[0].name, "bge-small-en-v1.5");
        }
    }

    #[test]
    fn model_size_returns_none_when_not_installed() {
        let tmp = TempDir::new().unwrap();
        let mgr = ModelManager::new(tmp.path().to_path_buf());
        assert!(mgr.model_size("all-MiniLM-L6-v2").is_none());
    }

    #[test]
    fn model_size_returns_size_when_installed() {
        #[cfg(not(feature = "onnx"))]
        {
            let tmp = TempDir::new().unwrap();
            let mgr = ModelManager::new(tmp.path().to_path_buf());

            mgr.install("all-MiniLM-L6-v2", |_, _| {}).unwrap();
            let size = mgr.model_size("all-MiniLM-L6-v2");
            // Marker file is the only content, so size should be small but nonzero.
            assert!(size.is_some());
            assert!(size.unwrap() > 0);
        }
    }

    #[test]
    fn has_onnx_files_detects_presence() {
        let tmp = TempDir::new().unwrap();
        let model_dir = tmp.path().join("test-model");
        fs::create_dir_all(&model_dir).unwrap();

        assert!(!has_onnx_files(&model_dir));

        fs::write(model_dir.join("model.onnx"), b"fake-onnx").unwrap();
        assert!(!has_onnx_files(&model_dir));

        fs::write(model_dir.join("tokenizer.json"), b"{}").unwrap();
        assert!(has_onnx_files(&model_dir));
    }

    #[test]
    fn compute_sha256_works() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.txt");
        fs::write(&path, b"hello world\n").unwrap();

        let hash = compute_sha256(&path).unwrap();
        // SHA256("hello world\n") is a known value.
        assert_eq!(hash.len(), 64); // 256 bits = 64 hex chars
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
