//! Upgrade support for ggnmem.
//!
//! `ggnmem upgrade`                — upgrade from a local release bundle.
//! `ggnmem upgrade --bundle PATH`  — upgrade from a specific tarball or directory.
//!
//! This replaces binaries in `~/.local/bin/` while preserving config and database.
//!
//! Phase 17 enhancements:
//! - SHA256 checksum verification from `checksums.txt` in the bundle
//! - Rollback to `.old` backups if verification fails after replacement
//! - Reports on preserved config, database, and installed AI models

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

fn models_dir() -> Result<PathBuf> {
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().unwrap_or_default().join(".local").join("share"));
    Ok(data_home.join("ggnmem").join("models"))
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

// ─── Checksum verification ──────────────────────────────────────────────────

/// Verify bundle integrity using checksums.txt if present.
///
/// Returns Ok(true) if checksums verified, Ok(false) if no checksums.txt,
/// or Err if verification failed.
fn verify_bundle_checksums(bundle_dir: &Path) -> Result<bool> {
    let checksums_path = bundle_dir.join("checksums.txt");
    if !checksums_path.exists() {
        return Ok(false);
    }

    let contents = fs::read_to_string(&checksums_path).context("read checksums.txt")?;
    let mut all_ok = true;

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Format: "<sha256>  <filename>" (two spaces between hash and name).
        let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
        if parts.len() != 2 {
            continue;
        }

        let expected_hash = parts[0].trim();
        let filename = parts[1].trim();
        let file_path = bundle_dir.join(filename);

        if !file_path.exists() {
            println!("  ⚠ {filename}: file missing (skipped)");
            continue;
        }

        // Compute SHA256 using sha256sum command.
        let output = Command::new("sha256sum")
            .arg(&file_path)
            .output()
            .with_context(|| format!("sha256sum {filename}"))?;

        if output.status.success() {
            let actual = String::from_utf8_lossy(&output.stdout);
            let actual_hash = actual.split_whitespace().next().unwrap_or("");
            if actual_hash == expected_hash {
                println!("  ✓ {filename}: checksum OK");
            } else {
                println!("  ✗ {filename}: checksum MISMATCH");
                println!("    expected: {expected_hash}");
                println!("    actual:   {actual_hash}");
                all_ok = false;
            }
        } else {
            println!("  ⚠ {filename}: could not compute hash");
        }
    }

    if !all_ok {
        bail!("bundle checksum verification failed — aborting upgrade");
    }

    Ok(true)
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

// ─── Rollback ────────────────────────────────────────────────────────────────

/// Restore backed-up binaries after a failed upgrade.
fn rollback(target_dir: &Path) {
    println!();
    println!("  ⚠ rolling back to previous binaries...");

    let cli_backup = target_dir.join("ggnmem.old");
    let daemon_backup = target_dir.join("ggnmem-daemon.old");
    let cli_target = target_dir.join("ggnmem");
    let daemon_target = target_dir.join("ggnmem-daemon");

    if cli_backup.exists() {
        if fs::copy(&cli_backup, &cli_target).is_ok() {
            println!("  ✓ restored ggnmem from backup");
        } else {
            println!("  ✗ failed to restore ggnmem — manual fix needed");
        }
    }

    if daemon_backup.exists() {
        if fs::copy(&daemon_backup, &daemon_target).is_ok() {
            println!("  ✓ restored ggnmem-daemon from backup");
        } else {
            println!("  ✗ failed to restore ggnmem-daemon — manual fix needed");
        }
    }
}

fn print_help() {
    println!("ggnmem upgrade — update ggnmem from a release bundle\n");
    println!("usage: ggnmem upgrade [--bundle <PATH>]\n");
    println!("options:");
    println!("  --bundle <PATH>    Path to release bundle directory or tarball");
    println!("  --help, -h         Show this help message\n");
    println!("This command will validate the bundle, backup existing binaries,");
    println!("and securely replace them with the new versions.");
}

// ─── Main upgrade command ────────────────────────────────────────────────────

pub fn cmd_upgrade(args: &[String]) -> Result<()> {
    if crate::has_flag(args, "--help") || crate::has_flag(args, "-h") {
        print_help();
        return Ok(());
    }

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

    // ── Validate bundle checksums ───────────────────────────────────────

    println!();
    print!("  validating bundle ... ");
    match verify_bundle_checksums(&bundle_dir) {
        Ok(true) => println!("checksums verified"),
        Ok(false) => println!("no checksums.txt (skipped)"),
        Err(e) => {
            println!("FAILED");
            return Err(e);
        }
    }

    // Show current version.
    let current_version = env!("CARGO_PKG_VERSION");
    println!();
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

    // Copy new binaries. (Remove existing to avoid 'Text file busy' os error 26)
    if target_cli.exists() {
        let _ = fs::remove_file(&target_cli);
    }
    fs::copy(&new_cli, &target_cli)
        .with_context(|| format!("copy ggnmem to {}", target_cli.display()))?;
    set_executable(&target_cli)?;
    println!("  ✓ installed ggnmem");

    if target_daemon.exists() {
        let _ = fs::remove_file(&target_daemon);
    }
    fs::copy(&new_daemon, &target_daemon)
        .with_context(|| format!("copy ggnmem-daemon to {}", target_daemon.display()))?;
    set_executable(&target_daemon)?;
    println!("  ✓ installed ggnmem-daemon");

    // ── Verify installed binaries ───────────────────────────────────────

    println!();
    print!("  verifying ... ");
    match get_binary_version(&target_cli) {
        Some(installed_version) => {
            println!("{installed_version}");
        }
        None => {
            println!("FAILED — could not run installed binary");
            rollback(&target_dir);
            bail!("upgrade verification failed — rolled back to previous version");
        }
    }

    // ── Preserve notices ────────────────────────────────────────────────

    println!();
    println!("  preserved:");
    println!("  ✓ config   (~/.config/ggnmem/config.toml)");
    println!("  ✓ database (~/.local/share/ggnmem/ggnmem.db)");

    // Report on installed AI models.
    match models_dir() {
        Ok(mdir) if mdir.exists() => {
            let mut model_count = 0;
            if let Ok(entries) = fs::read_dir(&mdir) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        model_count += 1;
                    }
                }
            }
            if model_count > 0 {
                println!(
                    "  ✓ models   (~/.local/share/ggnmem/models/ — {} model{})",
                    model_count,
                    if model_count == 1 { "" } else { "s" }
                );
            }
        }
        _ => {}
    }

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
