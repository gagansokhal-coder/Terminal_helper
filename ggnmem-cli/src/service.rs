//! Daemon lifecycle management for ggnmem.
//!
//! `ggnmem start`     — spawn ggnmem-daemon in background, write PID file.
//! `ggnmem stop`      — read PID file, send SIGTERM, clean up.
//! `ggnmem restart`   — stop + start.
//! `ggnmem autostart enable`  — systemd user service or shell rc fallback.
//! `ggnmem autostart disable` — remove autostart configuration.

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

fn state_dir() -> Result<PathBuf> {
    Ok(home_dir()?.join(".local").join("state").join("ggnmem"))
}

fn pid_path() -> Result<PathBuf> {
    Ok(state_dir()?.join("daemon.pid"))
}

fn daemon_binary() -> String {
    // Check ~/.local/bin first, then fall back to PATH.
    if let Ok(home) = home_dir() {
        let local_bin = home.join(".local").join("bin").join("ggnmem-daemon");
        if local_bin.exists() {
            return local_bin.to_string_lossy().into_owned();
        }
    }
    "ggnmem-daemon".to_owned()
}

// ─── PID management ──────────────────────────────────────────────────────────

fn read_pid() -> Result<Option<u32>> {
    let path = pid_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let contents = fs::read_to_string(&path)
        .with_context(|| format!("read PID file: {}", path.display()))?;
    let pid: u32 = contents
        .trim()
        .parse()
        .with_context(|| format!("invalid PID in {}: '{}'", path.display(), contents.trim()))?;
    Ok(Some(pid))
}

fn write_pid(pid: u32) -> Result<()> {
    let path = pid_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create state dir: {}", parent.display()))?;
    }
    fs::write(&path, pid.to_string())
        .with_context(|| format!("write PID file: {}", path.display()))?;
    Ok(())
}

fn remove_pid() -> Result<()> {
    let path = pid_path()?;
    if path.exists() {
        fs::remove_file(&path)
            .with_context(|| format!("remove PID file: {}", path.display()))?;
    }
    Ok(())
}

/// Check if a process with the given PID is still running.
fn is_process_running(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

// ─── Start ───────────────────────────────────────────────────────────────────

pub fn cmd_start() -> Result<()> {
    // Check if already running.
    if let Some(pid) = read_pid()? {
        if is_process_running(pid) {
            println!("  ✓ daemon already running (PID {pid})");
            return Ok(());
        }
        // Stale PID file — clean it up.
        remove_pid()?;
    }

    let binary = daemon_binary();
    println!("  starting daemon: {binary}");

    let child = Command::new(&binary)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null())
        .spawn()
        .with_context(|| format!("spawn daemon: {binary}"))?;

    let pid = child.id();
    write_pid(pid)?;

    // Give the daemon a moment to start, then verify it's running.
    std::thread::sleep(std::time::Duration::from_millis(500));

    if is_process_running(pid) {
        println!("  ✓ daemon started (PID {pid})");
    } else {
        remove_pid()?;
        bail!("daemon exited immediately after start — check logs");
    }

    Ok(())
}

// ─── Stop ────────────────────────────────────────────────────────────────────

pub fn cmd_stop() -> Result<()> {
    match read_pid()? {
        Some(pid) => {
            if !is_process_running(pid) {
                remove_pid()?;
                println!("  ✗ daemon not running (stale PID {pid} cleaned up)");
                return Ok(());
            }

            // Send SIGTERM via kill command (safe, no unsafe code).
            let status = Command::new("kill")
                .arg(pid.to_string())
                .status()
                .context("send SIGTERM to daemon")?;

            if status.success() {
                // Wait briefly for the process to exit.
                for _ in 0..10 {
                    if !is_process_running(pid) {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(200));
                }
                remove_pid()?;
                println!("  ✓ daemon stopped (PID {pid})");
            } else {
                println!("  ✗ failed to stop daemon (PID {pid})");
                println!("    try: kill -9 {pid}");
            }
            Ok(())
        }
        None => {
            println!("  ✗ daemon not running (no PID file)");
            Ok(())
        }
    }
}

// ─── Restart ─────────────────────────────────────────────────────────────────

pub fn cmd_restart() -> Result<()> {
    cmd_stop()?;
    println!();
    cmd_start()
}

// ─── Status (PID-aware) ─────────────────────────────────────────────────────

/// Check daemon status from PID file. Returns (running, pid).
pub fn daemon_status() -> Result<(bool, Option<u32>)> {
    match read_pid()? {
        Some(pid) => {
            let running = is_process_running(pid);
            if !running {
                // Clean up stale PID.
                let _ = remove_pid();
            }
            Ok((running, Some(pid)))
        }
        None => Ok((false, None)),
    }
}

// ─── Autostart ───────────────────────────────────────────────────────────────

const SYSTEMD_SERVICE: &str = r#"[Unit]
Description=ggnmem background daemon
Documentation=https://github.com/ggnmem/ggnmem
After=default.target

[Service]
Type=simple
ExecStart=%h/.local/bin/ggnmem-daemon
Restart=on-failure
RestartSec=5
Environment=XDG_RUNTIME_DIR=%t

[Install]
WantedBy=default.target
"#;

const AUTOSTART_MARKER: &str = "# ggnmem daemon autostart";
const AUTOSTART_MARKER_END: &str = "# end ggnmem daemon autostart";

fn systemd_service_path() -> Result<PathBuf> {
    Ok(home_dir()?
        .join(".config")
        .join("systemd")
        .join("user")
        .join("ggnmem-daemon.service"))
}

fn has_systemd() -> bool {
    // Check if systemctl --user is available.
    // Note: is-system-running can return non-zero on degraded systems,
    // so we just check if the command exists and runs at all.
    Command::new("systemctl")
        .args(["--user", "status"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

pub fn cmd_autostart_enable() -> Result<()> {
    if has_systemd() {
        enable_systemd()?;
    } else {
        enable_shell_fallback()?;
    }
    Ok(())
}

pub fn cmd_autostart_disable() -> Result<()> {
    if has_systemd() {
        disable_systemd()?;
    }
    // Always clean shell rc too, in case both were set.
    disable_shell_fallback()?;
    Ok(())
}

fn enable_systemd() -> Result<()> {
    let service_path = systemd_service_path()?;
    if let Some(parent) = service_path.parent() {
        fs::create_dir_all(parent).context("create systemd user dir")?;
    }

    fs::write(&service_path, SYSTEMD_SERVICE)
        .with_context(|| format!("write service file: {}", service_path.display()))?;
    println!("  ✓ service file: {}", service_path.display());

    // daemon-reload + enable.
    let reload = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status()
        .context("systemctl daemon-reload")?;
    if !reload.success() {
        bail!("systemctl daemon-reload failed");
    }

    let enable = Command::new("systemctl")
        .args(["--user", "enable", "ggnmem-daemon.service"])
        .status()
        .context("systemctl enable")?;
    if !enable.success() {
        bail!("systemctl enable failed");
    }

    println!("  ✓ systemd user service enabled");
    println!();
    println!("  start now: systemctl --user start ggnmem-daemon");
    println!("  check:     systemctl --user status ggnmem-daemon");

    Ok(())
}

fn disable_systemd() -> Result<()> {
    // Stop first.
    let _ = Command::new("systemctl")
        .args(["--user", "stop", "ggnmem-daemon.service"])
        .status();

    let _ = Command::new("systemctl")
        .args(["--user", "disable", "ggnmem-daemon.service"])
        .status();

    let service_path = systemd_service_path()?;
    if service_path.exists() {
        fs::remove_file(&service_path)
            .with_context(|| format!("remove service file: {}", service_path.display()))?;
        println!("  ✓ systemd service removed");
    }

    let _ = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status();

    Ok(())
}

fn enable_shell_fallback() -> Result<()> {
    println!("  ⚠ systemd not available, using shell startup fallback");

    let shell = std::env::var("SHELL").unwrap_or_default();
    let rc_path = if shell.contains("zsh") {
        home_dir()?.join(".zshrc")
    } else {
        home_dir()?.join(".bashrc")
    };

    // Check if already present.
    if rc_path.exists() {
        let contents = fs::read_to_string(&rc_path)?;
        if contents.contains(AUTOSTART_MARKER) {
            println!("  ✓ autostart already configured in {}", rc_path.display());
            return Ok(());
        }
    }

    let block = format!(
        "\n{AUTOSTART_MARKER}\n\
         if ! pgrep -x ggnmem-daemon > /dev/null 2>&1; then\n    \
             ggnmem-daemon &>/dev/null & disown 2>/dev/null\n\
         fi\n\
         {AUTOSTART_MARKER_END}\n"
    );

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&rc_path)
        .with_context(|| format!("open {} for append", rc_path.display()))?;

    use std::io::Write;
    file.write_all(block.as_bytes())?;

    println!("  ✓ autostart added to {}", rc_path.display());
    Ok(())
}

fn disable_shell_fallback() -> Result<()> {
    for rc_name in &[".bashrc", ".zshrc"] {
        let rc_path = home_dir()?.join(rc_name);
        if !rc_path.exists() {
            continue;
        }

        let contents = fs::read_to_string(&rc_path)?;
        if !contents.contains(AUTOSTART_MARKER) {
            continue;
        }

        let mut output = String::with_capacity(contents.len());
        let mut in_block = false;

        for line in contents.lines() {
            if line.contains(AUTOSTART_MARKER) && !line.contains(AUTOSTART_MARKER_END) {
                in_block = true;
                continue;
            }
            if line.contains(AUTOSTART_MARKER_END) {
                in_block = false;
                continue;
            }
            if in_block {
                continue;
            }
            output.push_str(line);
            output.push('\n');
        }

        fs::write(&rc_path, &output)?;
        println!("  ✓ autostart removed from {}", rc_path.display());
    }

    Ok(())
}
