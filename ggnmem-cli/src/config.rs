//! User configuration system for ggnmem.
//!
//! Reads and writes `~/.config/ggnmem/config.toml`.
//! Provides `ggnmem config show` and `ggnmem config set KEY VALUE`.

use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

// ─── Config struct ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GgnmemConfig {
    #[serde(default = "default_features")]
    pub features: FeaturesConfig,

    #[serde(default = "default_daemon")]
    pub daemon: DaemonSection,

    #[serde(default = "default_appearance")]
    pub appearance: AppearanceConfig,

    #[serde(default = "default_limits")]
    pub limits: LimitsConfig,

    #[serde(default = "default_search")]
    pub search: SearchConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturesConfig {
    #[serde(default = "bool_true")]
    pub capture: bool,

    #[serde(default = "bool_true")]
    pub search: bool,

    #[serde(default = "bool_true")]
    pub tui: bool,

    #[serde(default)]
    pub ai: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonSection {
    #[serde(default)]
    pub autostart: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceConfig {
    #[serde(default = "default_theme")]
    pub theme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitsConfig {
    #[serde(default = "default_max_history")]
    pub max_history: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    #[serde(default = "default_index_mode")]
    pub index_mode: String,
}

// ─── Defaults ────────────────────────────────────────────────────────────────

fn bool_true() -> bool {
    true
}
fn default_theme() -> String {
    "auto".to_owned()
}
fn default_max_history() -> u64 {
    100_000
}
fn default_index_mode() -> String {
    "balanced".to_owned()
}
fn default_features() -> FeaturesConfig {
    FeaturesConfig {
        capture: true,
        search: true,
        tui: true,
        ai: false,
    }
}
fn default_daemon() -> DaemonSection {
    DaemonSection { autostart: false }
}
fn default_appearance() -> AppearanceConfig {
    AppearanceConfig {
        theme: default_theme(),
    }
}
fn default_limits() -> LimitsConfig {
    LimitsConfig {
        max_history: default_max_history(),
    }
}
fn default_search() -> SearchConfig {
    SearchConfig {
        index_mode: default_index_mode(),
    }
}

impl Default for GgnmemConfig {
    fn default() -> Self {
        Self {
            features: default_features(),
            daemon: default_daemon(),
            appearance: default_appearance(),
            limits: default_limits(),
            search: default_search(),
        }
    }
}

// ─── Path helpers ────────────────────────────────────────────────────────────

pub fn config_path() -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is not set")?;
    Ok(home.join(".config").join("ggnmem").join("config.toml"))
}

// ─── Load / Save ─────────────────────────────────────────────────────────────

pub fn load() -> Result<GgnmemConfig> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(GgnmemConfig::default());
    }
    let contents =
        fs::read_to_string(&path).with_context(|| format!("read config: {}", path.display()))?;
    let config: GgnmemConfig =
        toml::from_str(&contents).with_context(|| format!("parse config: {}", path.display()))?;
    Ok(config)
}

pub fn save(config: &GgnmemConfig) -> Result<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create config dir: {}", parent.display()))?;
    }
    let contents = toml::to_string_pretty(config).context("serialize config")?;
    // Add a header comment.
    let output = format!(
        "# ggnmem configuration\n# See: https://github.com/ggnmem/ggnmem\n\n{contents}"
    );
    fs::write(&path, output).with_context(|| format!("write config: {}", path.display()))?;
    Ok(())
}

// ─── CLI commands ────────────────────────────────────────────────────────────

/// `ggnmem config show` — pretty-print current config.
pub fn cmd_show() -> Result<()> {
    let path = config_path()?;
    let config = load()?;

    println!("ggnmem config");
    println!("─────────────────────────────────");
    if path.exists() {
        println!("  file: {}", path.display());
    } else {
        println!("  file: {} (using defaults)", path.display());
    }
    println!();

    println!("  [features]");
    println!("    capture      = {}", config.features.capture);
    println!("    search       = {}", config.features.search);
    println!("    tui          = {}", config.features.tui);
    println!("    ai           = {}", config.features.ai);
    println!();
    println!("  [daemon]");
    println!("    autostart    = {}", config.daemon.autostart);
    println!();
    println!("  [appearance]");
    println!("    theme        = \"{}\"", config.appearance.theme);
    println!();
    println!("  [limits]");
    println!("    max_history  = {}", config.limits.max_history);
    println!();
    println!("  [search]");
    println!("    index_mode   = \"{}\"", config.search.index_mode);

    Ok(())
}

/// `ggnmem config set KEY VALUE` — update a single config key.
pub fn cmd_set(args: &[String]) -> Result<()> {
    let key = args
        .get(3)
        .context("usage: ggnmem config set <key> <value>")?;
    let value = args
        .get(4)
        .context("usage: ggnmem config set <key> <value>")?;

    let mut config = load()?;

    match key.as_str() {
        "capture" => config.features.capture = parse_bool(value)?,
        "search" => config.features.search = parse_bool(value)?,
        "tui" => config.features.tui = parse_bool(value)?,
        "ai" => config.features.ai = parse_bool(value)?,
        "autostart" | "daemon_autostart" => config.daemon.autostart = parse_bool(value)?,
        "theme" => config.appearance.theme = value.clone(),
        "max_history" => {
            config.limits.max_history = value
                .parse::<u64>()
                .context("max_history must be a positive number")?;
        }
        "index_mode" => {
            let mode = value.as_str();
            if !["lite", "balanced", "power"].contains(&mode) {
                bail!("index_mode must be one of: lite, balanced, power");
            }
            config.search.index_mode = value.clone();
        }
        other => {
            bail!(
                "unknown config key: {other}\n\navailable keys:\n  capture, search, tui, ai, autostart, theme, max_history, index_mode"
            );
        }
    }

    save(&config)?;
    println!("  ✓ {key} = {value}");
    println!("  saved to {}", config_path()?.display());

    Ok(())
}

fn parse_bool(value: &str) -> Result<bool> {
    match value.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => bail!("expected boolean value (true/false), got: {value}"),
    }
}
