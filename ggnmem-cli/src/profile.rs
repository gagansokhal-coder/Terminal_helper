//! Profile presets for ggnmem.
//!
//! `ggnmem profile list`        — show available profiles.
//! `ggnmem profile apply <name>` — apply a profile to config.toml.

use anyhow::{bail, Result};

use crate::config::{self, GgnmemConfig};

// ─── Profile definitions ─────────────────────────────────────────────────────

struct Profile {
    name: &'static str,
    description: &'static str,
    detail: &'static str,
    apply: fn(&mut GgnmemConfig),
}

const PROFILES: &[Profile] = &[
    Profile {
        name: "lite",
        description: "Low RAM, capture only",
        detail: "capture=true, search=false, tui=false, ai=false, max_history=10000",
        apply: apply_lite,
    },
    Profile {
        name: "balanced",
        description: "Default — search + TUI enabled",
        detail: "capture=true, search=true, tui=true, ai=false, max_history=100000",
        apply: apply_balanced,
    },
    Profile {
        name: "power",
        description: "High indexing, future AI ready",
        detail: "capture=true, search=true, tui=true, ai=false, max_history=500000",
        apply: apply_power,
    },
];

fn apply_lite(cfg: &mut GgnmemConfig) {
    cfg.features.capture = true;
    cfg.features.search = false;
    cfg.features.tui = false;
    cfg.features.ai = false;
    cfg.limits.max_history = 10_000;
    cfg.search.index_mode = "lite".to_owned();
}

fn apply_balanced(cfg: &mut GgnmemConfig) {
    cfg.features.capture = true;
    cfg.features.search = true;
    cfg.features.tui = true;
    cfg.features.ai = false;
    cfg.limits.max_history = 100_000;
    cfg.search.index_mode = "balanced".to_owned();
}

fn apply_power(cfg: &mut GgnmemConfig) {
    cfg.features.capture = true;
    cfg.features.search = true;
    cfg.features.tui = true;
    cfg.features.ai = false;
    cfg.limits.max_history = 500_000;
    cfg.search.index_mode = "power".to_owned();
}

// ─── CLI commands ────────────────────────────────────────────────────────────

/// `ggnmem profile list` — show available profiles.
pub fn cmd_list() -> Result<()> {
    println!("ggnmem profiles");
    println!("─────────────────────────────────");
    println!();

    for p in PROFILES {
        println!("  {} — {}", p.name, p.description);
        println!("    {}", p.detail);
        println!();
    }

    // Show which profile is closest to current config.
    let config = config::load()?;
    if let Some(name) = detect_profile(&config) {
        println!("  active: {name}");
    } else {
        println!("  active: custom");
    }

    Ok(())
}

/// `ggnmem profile apply <name>` — apply a named profile.
pub fn cmd_apply(args: &[String]) -> Result<()> {
    let name = args
        .get(3)
        .map(String::as_str)
        .unwrap_or("");

    let profile = PROFILES
        .iter()
        .find(|p| p.name == name);

    match profile {
        Some(p) => {
            let mut config = config::load()?;
            (p.apply)(&mut config);
            config::save(&config)?;
            println!("  ✓ profile '{}' applied", p.name);
            println!("    {}", p.detail);
            println!("  saved to {}", config::config_path()?.display());
            Ok(())
        }
        None => {
            let names: Vec<&str> = PROFILES.iter().map(|p| p.name).collect();
            bail!(
                "unknown profile: '{}'\n\navailable profiles: {}",
                name,
                names.join(", ")
            );
        }
    }
}

/// Detect which predefined profile matches the current config (if any).
pub fn detect_profile(cfg: &GgnmemConfig) -> Option<&'static str> {
    if !cfg.features.capture {
        return None;
    }

    if !cfg.features.search && !cfg.features.tui && !cfg.features.ai && cfg.limits.max_history <= 10_000
    {
        return Some("lite");
    }

    if cfg.features.search
        && cfg.features.tui
        && !cfg.features.ai
        && cfg.limits.max_history >= 90_000
        && cfg.limits.max_history <= 110_000
    {
        return Some("balanced");
    }

    if cfg.features.search
        && cfg.features.tui
        && cfg.limits.max_history >= 400_000
    {
        return Some("power");
    }

    None
}
