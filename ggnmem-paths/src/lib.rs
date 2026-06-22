//! Cross-platform path resolution for ggnmem.
//!
//! This crate centralizes all logic for discovering where ggnmem stores
//! its configuration, data, logs, models, and state on different platforms.
//!
//! Windows:
//! - Config: `%APPDATA%\ggnmem`
//! - Knowledge: `%APPDATA%\ggnmem\knowledge`
//! - Data: `%LOCALAPPDATA%\ggnmem\data`
//! - Logs: `%LOCALAPPDATA%\ggnmem\logs`
//! - State: `%LOCALAPPDATA%\ggnmem`
//! - Models: `%LOCALAPPDATA%\ggnmem\ai\models`
//! - Binaries: `%LOCALAPPDATA%\ggnmem\bin`
//!
//! Unix:
//! Uses standard XDG base directories with fallback to `~/.config`,
//! `~/.local/share`, `~/.local/state`, etc.

use std::path::PathBuf;

/// Gets the user's home directory.
///
/// On Windows, this is `%USERPROFILE%`.
/// On Unix, this is `$HOME`.
pub fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    }
    #[cfg(unix)]
    {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}

/// Gets the configuration directory.
///
/// Windows: `%APPDATA%\ggnmem`
/// Unix: `$XDG_CONFIG_HOME/ggnmem` or `~/.config/ggnmem`
pub fn config_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA").map(|s| PathBuf::from(s).join("ggnmem"))
    }
    #[cfg(unix)]
    {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            Some(PathBuf::from(xdg).join("ggnmem"))
        } else {
            home_dir().map(|home| home.join(".config").join("ggnmem"))
        }
    }
}

/// Gets the user knowledge packs directory.
///
/// Windows: `%APPDATA%\ggnmem\knowledge`
/// Unix: `$XDG_CONFIG_HOME/ggnmem/knowledge` or `~/.config/ggnmem/knowledge`
pub fn knowledge_dir() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("knowledge"))
}

/// Gets the base data directory.
///
/// Windows: `%LOCALAPPDATA%\ggnmem`
/// Unix: `$XDG_DATA_HOME/ggnmem` or `~/.local/share/ggnmem`
fn base_data_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("LOCALAPPDATA").map(|s| PathBuf::from(s).join("ggnmem"))
    }
    #[cfg(unix)]
    {
        if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
            Some(PathBuf::from(xdg).join("ggnmem"))
        } else {
            home_dir().map(|home| home.join(".local").join("share").join("ggnmem"))
        }
    }
}

/// Gets the database and data directory.
///
/// Windows: `%LOCALAPPDATA%\ggnmem\data`
/// Unix: `$XDG_DATA_HOME/ggnmem` or `~/.local/share/ggnmem`
pub fn data_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        base_data_dir().map(|dir| dir.join("data"))
    }
    #[cfg(unix)]
    {
        base_data_dir()
    }
}

/// Gets the AI models directory.
///
/// Windows: `%LOCALAPPDATA%\ggnmem\ai\models`
/// Unix: `$XDG_DATA_HOME/ggnmem/models` or `~/.local/share/ggnmem/models`
pub fn models_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        base_data_dir().map(|dir| dir.join("ai").join("models"))
    }
    #[cfg(unix)]
    {
        base_data_dir().map(|dir| dir.join("models"))
    }
}

/// Gets the state and PID directory.
///
/// Windows: `%LOCALAPPDATA%\ggnmem`
/// Unix: `$XDG_STATE_HOME/ggnmem` or `~/.local/state/ggnmem`
pub fn state_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("LOCALAPPDATA").map(|s| PathBuf::from(s).join("ggnmem"))
    }
    #[cfg(unix)]
    {
        if let Some(xdg) = std::env::var_os("XDG_STATE_HOME") {
            Some(PathBuf::from(xdg).join("ggnmem"))
        } else {
            home_dir().map(|home| home.join(".local").join("state").join("ggnmem"))
        }
    }
}

/// Gets the logs directory.
///
/// Windows: `%LOCALAPPDATA%\ggnmem\logs`
/// Unix: `$XDG_STATE_HOME/ggnmem/logs` or `~/.local/state/ggnmem/logs`
pub fn logs_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        state_dir().map(|dir| dir.join("logs"))
    }
    #[cfg(unix)]
    {
        state_dir().map(|dir| dir.join("logs"))
    }
}

/// Gets the bin directory.
///
/// Windows: `%LOCALAPPDATA%\ggnmem\bin`
/// Unix: `~/.local/bin`
pub fn bin_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("LOCALAPPDATA").map(|s| PathBuf::from(s).join("ggnmem").join("bin"))
    }
    #[cfg(unix)]
    {
        home_dir().map(|home| home.join(".local").join("bin"))
    }
}
