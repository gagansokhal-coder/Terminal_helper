//! Upgrade support for ggnmem.
//!
//! `ggnmem upgrade`                — upgrade from a local release bundle.
//! `ggnmem upgrade --bundle PATH`  — upgrade from a specific tarball or directory.
//!
//! This replaces binaries in `~/.local/bin/` while preserving config and database.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

// ─── Path helpers ────────────────────────────────────────────────────────────

fn home_dir() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is not set")
}

fn bin_dir() -> Result<PathBuf> {
    Ok(home_dir()?.join(".local").join("bin"))
}

// ─── Bundle discovery ────────────────────────────────────────────────────────

/// Look for a release bundle in common locations.
fn find_bundle(explicit: Option<&str>) -> Result<PathBuf> {
    // Explicit path from --bundle flag.
    if let Some(path_str) = explicit {
        let path = PathBuf::from(path_str);
        if path.is_dir() {
            return Ok(path);
        }
        if path.is_file() && path_str.ends_with(".tar.gz") {
            return extract_tarball(&path);
        }
        bail!("bundle path does not exist or is not a directory/tarball: {path_str}");
    }

    // Auto-discover: look in common locations.
    let candidates = discover_candidates();
    for candidate in &candidates {
        if candidate.join("ggnmem").exists() && candidate.join("ggnmem-daemon").exists() {
            return Ok(candidate.clone());
        }
    }

    bail!(
        "no release bundle found.\n\n\
         Provide a path explicitly:\n  \
         ggnmem upgrade --bundle ./release\n  \
         ggnmem upgrade --bundle ggnmem-linux-x86_64.tar.gz\n\n\
         Or place release binaries in one of these locations:\n  \
         ./release/\n  \
         ../release/"
    );
}

fn discover_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    // Current directory release/.
    candidates.push(PathBuf::from("./release"));

    // Parent directory release/ (common when running from project root).
    candidates.push(PathBuf::from("../release"));

    // Beside the currently running CLI binary.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("release"));
            // Also check sibling files directly (when extracted tarball is flat).
            candidates.push(parent.to_path_buf());
        }
    }

    candidates
}

/// Extract a .tar.gz file to a temporary directory and return the path.
fn extract_tarball(tarball: &Path) -> Result<PathBuf> {
    let extract_dir = std::env::temp_dir().join("ggnmem-upgrade");
    if extract_dir.exists() {
        fs::remove_dir_all(&extract_dir).context("clean previous extract dir")?;
    }
    fs::create_dir_all(&extract_dir).context("create extract dir")?;

    println!("  extracting {}...", tarball.display());
    let status = Command::new("tar")
        .args(["xzf", &tarball.to_string_lossy(), "-C"])
        .arg(&extract_dir)
        .status()
        .context("run tar to extract bundle")?;

    if !status.success() {
        bail!("tar extraction failed");
    }

    // The tarball might extract into a subdirectory or flat.
    // Check for binaries directly in extract_dir first.
    if extract_dir.join("ggnmem").exists() {
        return Ok(extract_dir);
    }

    // Check one level deep.
    if let Ok(entries) = fs::read_dir(&extract_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("ggnmem").exists() {
                return Ok(path);
            }
        }
    }

    bail!(
        "extracted tarball but could not find ggnmem binary in {}",
        extract_dir.display()
    );
}

// ─── Version comparison ──────────────────────────────────────────────────────

/// Get the version string from a binary by running it with `version`.
fn get_binary_version(binary_path: &Path) -> Option<String> {
    Command::new(binary_path)
        .arg("version")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // The first line should be "ggnmem X.Y.Z-..."
                stdout.lines().next().map(|line| line.trim().to_owned())
            } else {
                None
            }
        })
}

// ─── Main upgrade command ────────────────────────────────────────────────────

pub fn cmd_upgrade(args: &[String]) -> Result<()> {
    println!("ggnmem upgrade");
    println!("─────────────────────────────────");
    println!();

    let bundle_path_arg = crate::parse_named_arg(args, "--bundle");
    let bundle_dir = find_bundle(bundle_path_arg.as_deref())?;

    let new_cli = bundle_dir.join("ggnmem");
    let new_daemon = bundle_dir.join("ggnmem-daemon");

    if !new_cli.exists() {
        bail!("ggnmem binary not found in bundle: {}", new_cli.display());
    }
    if !new_daemon.exists() {
        bail!(
            "ggnmem-daemon binary not found in bundle: {}",
            new_daemon.display()
        );
    }

    println!("  bundle: {}", bundle_dir.display());

    // Show current version.
    let current_version = env!("CARGO_PKG_VERSION");
    println!("  current version: {current_version}");

    // Show bundle version.
    if let Some(new_version) = get_binary_version(&new_cli) {
        println!("  bundle version:  {new_version}");
    } else {
        println!("  bundle version:  (could not determine)");
    }

    println!();

    // ── Stop daemon if running ──────────────────────────────────────────

    print!("  stopping daemon ... ");
    let daemon_was_running = match crate::service::daemon_status() {
        Ok((true, _)) => {
            match crate::service::cmd_stop() {
                Ok(()) => println!("done"),
                Err(e) => println!("warning: {e}"),
            }
            true
        }
        _ => {
            println!("not running");
            false
        }
    };

    // ── Copy binaries ───────────────────────────────────────────────────

    let target_dir = bin_dir()?;
    fs::create_dir_all(&target_dir)
        .with_context(|| format!("create bin dir: {}", target_dir.display()))?;

    let target_cli = target_dir.join("ggnmem");
    let target_daemon = target_dir.join("ggnmem-daemon");

    // Backup existing binaries.
    if target_cli.exists() {
        let backup = target_dir.join("ggnmem.old");
        fs::copy(&target_cli, &backup)
            .with_context(|| format!("backup {}", target_cli.display()))?;
        println!("  ✓ backed up ggnmem → ggnmem.old");
    }
    if target_daemon.exists() {
        let backup = target_dir.join("ggnmem-daemon.old");
        fs::copy(&target_daemon, &backup)
            .with_context(|| format!("backup {}", target_daemon.display()))?;
        println!("  ✓ backed up ggnmem-daemon → ggnmem-daemon.old");
    }

    // Copy new binaries.
    fs::copy(&new_cli, &target_cli)
        .with_context(|| format!("copy ggnmem to {}", target_cli.display()))?;
    set_executable(&target_cli)?;
    println!("  ✓ installed ggnmem");

    fs::copy(&new_daemon, &target_daemon)
        .with_context(|| format!("copy ggnmem-daemon to {}", target_daemon.display()))?;
    set_executable(&target_daemon)?;
    println!("  ✓ installed ggnmem-daemon");

    // ── Verify installed binaries ───────────────────────────────────────

    println!();
    print!("  verifying ... ");
    if let Some(installed_version) = get_binary_version(&target_cli) {
        println!("{installed_version}");
    } else {
        println!("warning: could not verify installed binary");
    }

    // ── Preserve notice ─────────────────────────────────────────────────

    println!();
    println!("  ✓ config preserved (~/.config/ggnmem/config.toml)");
    println!("  ✓ database preserved (~/.local/share/ggnmem/ggnmem.db)");

    // ── Restart daemon if it was running ────────────────────────────────

    if daemon_was_running {
        println!();
        print!("  restarting daemon ... ");
        match crate::service::cmd_start() {
            Ok(()) => {} // cmd_start prints its own message
            Err(e) => println!("warning: {e}"),
        }
    }

    // ── Summary ─────────────────────────────────────────────────────────

    println!();
    println!("  ═══════════════════════════════════════");
    println!("  ✓ upgrade complete");
    println!("  ═══════════════════════════════════════");
    println!();
    println!("  next steps:");
    println!("    ggnmem version     — verify the new version");
    println!("    ggnmem doctor      — check system health");

    Ok(())
}

/// Set the executable bit on a file (Linux/macOS).
fn set_executable(path: &Path) -> Result<()> {
    let status = Command::new("chmod")
        .args(["+x", &path.to_string_lossy()])
        .status()
        .with_context(|| format!("chmod +x {}", path.display()))?;
    if !status.success() {
        bail!("chmod +x failed for {}", path.display());
    }
    Ok(())
}
