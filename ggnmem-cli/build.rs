// Build script for ggnmem-cli.
//
// Captures compile-time metadata for `ggnmem version`:
//   - GGNMEM_BUILD_DATE      — ISO 8601 date (e.g. "2026-06-10")
//   - GGNMEM_GIT_COMMIT      — short git commit hash (e.g. "a1b2c3d")
//   - GGNMEM_BUILD_PROFILE   — "debug" or "release"
//   - GGNMEM_RUSTC_VERSION   — Rust compiler version (e.g. "1.82.0")
//   - GGNMEM_TARGET_PLATFORM — platform string (e.g. "linux-x86_64")

use std::process::Command;

fn main() {
    // ── Build date ───────────────────────────────────────────────────────
    let date = build_date();
    println!("cargo:rustc-env=GGNMEM_BUILD_DATE={date}");

    // ── Git commit ───────────────────────────────────────────────────────
    let commit = git_commit_short();
    println!("cargo:rustc-env=GGNMEM_GIT_COMMIT={commit}");

    // ── Build profile ────────────────────────────────────────────────────
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".to_owned());
    println!("cargo:rustc-env=GGNMEM_BUILD_PROFILE={profile}");

    // ── Rust compiler version ───────────────────────────────────────────
    let rustc_version = rustc_version();
    println!("cargo:rustc-env=GGNMEM_RUSTC_VERSION={rustc_version}");

    // ── Target platform ─────────────────────────────────────────────────
    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_owned());
    let platform = target_to_platform(&target);
    println!("cargo:rustc-env=GGNMEM_TARGET_PLATFORM={platform}");

    // Re-run only when git HEAD changes or this script changes.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/refs");
}

fn build_date() -> String {
    // Try system `date` first (works on Linux/macOS/WSL).
    if let Ok(output) = Command::new("date").arg("+%Y-%m-%d").output() {
        if output.status.success() {
            let date = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            if !date.is_empty() {
                return date;
            }
        }
    }

    // Fallback: use Rust's SystemTime (UTC, no chrono dependency).
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = now / 86400;
    let (year, month, day) = epoch_days_to_date(days);
    format!("{year:04}-{month:02}-{day:02}")
}

fn git_commit_short() -> String {
    if let Ok(output) = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
    {
        if output.status.success() {
            let hash = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            if !hash.is_empty() {
                return hash;
            }
        }
    }
    "unknown".to_owned()
}

/// Get the Rust compiler version string (e.g. "1.82.0").
fn rustc_version() -> String {
    if let Ok(output) = Command::new("rustc").arg("--version").output() {
        if output.status.success() {
            let full = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            // Output is "rustc 1.82.0 (..." — extract just the version number.
            if let Some(version) = full.strip_prefix("rustc ") {
                return version
                    .split_whitespace()
                    .next()
                    .unwrap_or("unknown")
                    .to_owned();
            }
        }
    }
    "unknown".to_owned()
}

/// Convert a Rust target triple to a user-friendly platform string.
///
/// Examples:
///   "x86_64-unknown-linux-gnu"    → "linux-x86_64"
///   "aarch64-unknown-linux-gnu"   → "linux-aarch64"
///   "x86_64-apple-darwin"         → "macos-x86_64"
///   "aarch64-apple-darwin"        → "macos-aarch64"
fn target_to_platform(target: &str) -> String {
    let parts: Vec<&str> = target.split('-').collect();
    if parts.len() < 3 {
        return target.to_owned();
    }

    let arch = parts[0];
    let os = if target.contains("linux") {
        "linux"
    } else if target.contains("darwin") {
        "macos"
    } else if target.contains("windows") {
        "windows"
    } else {
        parts[2]
    };

    format!("{os}-{arch}")
}

/// Convert epoch days to (year, month, day).
/// Algorithm from Howard Hinnant's chrono-compatible date library.
fn epoch_days_to_date(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
