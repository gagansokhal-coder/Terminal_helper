//! Daemon logging configuration.
//!
//! Writes structured logs to `~/.local/state/ggnmem/logs/daemon.log`.
//! Supports configurable log level via `GGNMEM_LOG_LEVEL` env var.
//! Performs size-based log rotation at startup.

use std::fs;
use std::path::PathBuf;

use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::EnvFilter;

// Default max log file size: 5 MB.
const DEFAULT_MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;
// Number of backup log files to keep.
const MAX_BACKUP_FILES: u32 = 3;

/// Resolve the log directory.
///
/// Windows: `%LOCALAPPDATA%\ggnmem\logs\`
/// Unix:    `~/.local/state/ggnmem/logs/`
pub fn log_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|dir| dir.join("ggnmem").join("logs"))
    }

    #[cfg(unix)]
    {
        std::env::var_os("HOME").map(PathBuf::from).map(|home| {
            home.join(".local")
                .join("state")
                .join("ggnmem")
                .join("logs")
        })
    }
}

/// Resolve the primary log file path.
pub fn log_path() -> Option<PathBuf> {
    log_dir().map(|dir| dir.join("daemon.log"))
}

/// Rotate the log file if it exceeds the size limit.
///
/// Rotation scheme: daemon.log → daemon.log.1 → daemon.log.2 → daemon.log.3 (dropped).
pub fn rotate_log_if_needed(max_bytes: u64) {
    let Some(log_file) = log_path() else {
        return;
    };

    if !log_file.exists() {
        return;
    }

    let size = fs::metadata(&log_file).map(|m| m.len()).unwrap_or(0);
    if size < max_bytes {
        return;
    }

    // Shift existing backups: .3 is dropped, .2 → .3, .1 → .2, current → .1.
    for i in (1..MAX_BACKUP_FILES).rev() {
        let from = log_file.with_extension(format!("log.{i}"));
        let to = log_file.with_extension(format!("log.{}", i + 1));
        if from.exists() {
            let _ = fs::rename(&from, &to);
        }
    }

    // Rotate current → .1
    let backup = log_file.with_extension("log.1");
    let _ = fs::rename(&log_file, &backup);
}

/// Parse the log level from `GGNMEM_LOG_LEVEL` env var.
/// Valid values: error, warn, info, debug, trace.
/// Returns a filter directive string.
fn log_level_filter() -> String {
    let level = std::env::var("GGNMEM_LOG_LEVEL")
        .unwrap_or_else(|_| "info".to_owned())
        .to_lowercase();

    match level.as_str() {
        "error" | "warn" | "info" | "debug" | "trace" => level,
        _ => "info".to_owned(),
    }
}

/// Initialize the logging system.
///
/// Sets up dual output:
/// - File: `~/.local/state/ggnmem/logs/daemon.log`
/// - Stderr: for systemd journal capture / debugging
///
/// Log level is controlled by `GGNMEM_LOG_LEVEL` env var (default: info).
pub fn init_logging() {
    let level = log_level_filter();

    // Rotate logs before opening the file.
    let max_bytes = std::env::var("GGNMEM_MAX_LOG_BYTES")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_MAX_LOG_BYTES);
    rotate_log_if_needed(max_bytes);

    // Build the env filter.
    let filter = EnvFilter::try_new(&level).unwrap_or_else(|_| EnvFilter::new("info"));

    // Try to set up file logging.
    if let Some(dir) = log_dir() {
        if fs::create_dir_all(&dir).is_ok() {
            let log_file = dir.join("daemon.log");
            if let Ok(file) = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_file)
            {
                // Dual writer: file (all levels) + stderr (warn+).
                let stderr_writer = std::io::stderr.with_max_level(tracing::Level::WARN);
                let file_writer = file;
                let writer = file_writer.and(stderr_writer);

                let _ = tracing_subscriber::fmt()
                    .with_writer(writer)
                    .with_target(false)
                    .with_ansi(false)
                    .compact()
                    .with_env_filter(filter)
                    .try_init();
                return;
            }
        }
    }

    // Fallback: stderr only.
    let _ = tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .with_env_filter(filter)
        .try_init();
}
