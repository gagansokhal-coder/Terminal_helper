// Build script for ggnmem-cli.
//
// Captures compile-time metadata for `ggnmem version`:
//   - GGNMEM_BUILD_DATE   — ISO 8601 date (e.g. "2026-06-10")
//   - GGNMEM_GIT_COMMIT   — short git commit hash (e.g. "a1b2c3d")
//   - GGNMEM_BUILD_PROFILE — "debug" or "release"

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
