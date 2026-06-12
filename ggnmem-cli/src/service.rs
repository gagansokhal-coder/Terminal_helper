//! Daemon lifecycle management for ggnmem.
//!
//! `ggnmem start`              — spawn ggnmem-daemon in background, write PID file.
//! `ggnmem stop`               — read PID file, send SIGTERM, clean up.
//! `ggnmem restart`            — stop + start.
//! `ggnmem logs`               — show daemon log output.
//! `ggnmem autostart enable`   — systemd user service or shell rc fallback.
//! `ggnmem autostart disable`  — remove autostart configuration.
//! `ggnmem autostart status`   — check autostart state.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
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

fn log_dir() -> Result<PathBuf> {
    Ok(state_dir()?.join("logs"))
}

fn log_path() -> Result<PathBuf> {
    Ok(log_dir()?.join("daemon.log"))
}

fn socket_path() -> Option<PathBuf> {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("ggnmem").join("daemon.sock"))
}

fn daemon_binary() -> String {
    // Prefer the daemon installed beside the currently running CLI. This keeps
    // bincode IPC structs aligned when multiple ggnmem installs are on PATH.
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(bin_dir) = current_exe.parent() {
            let sibling = bin_dir.join("ggnmem-daemon");
            if sibling.exists() {
                return sibling.to_string_lossy().into_owned();
            }
        }
    }

    // Check ~/.local/bin next, then fall back to PATH.
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
    let contents =
        fs::read_to_string(&path).with_context(|| format!("read PID file: {}", path.display()))?;
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
        fs::remove_file(&path).with_context(|| format!("remove PID file: {}", path.display()))?;
    }
    Ok(())
}

/// Check if the daemon is alive by probing the advisory lock on the PID file.
///
/// The daemon holds an exclusive `fs2` lock on `daemon.pid` for its entire
/// lifetime.  If we can acquire a shared lock, no daemon holds the exclusive
/// lock ⇒ the PID is stale.  If the attempt returns `WouldBlock`, the daemon
/// is running.
fn is_process_running(_pid: u32) -> bool {
    let path = match pid_path() {
        Ok(p) => p,
        Err(_) => return false,
    };
    if !path.exists() {
        return false;
    }
    let file = match fs::File::open(&path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    match fs2::FileExt::try_lock_shared(&file) {
        Ok(()) => {
            // We got the lock ⇒ nobody holds an exclusive lock ⇒ not running.
            let _ = fs2::FileExt::unlock(&file);
            false
        }
        Err(e) if e.kind() == fs2::lock_contended_error().kind() => {
            // Daemon holds the exclusive lock ⇒ running.
            true
        }
        Err(_) => {
            // Other I/O error — assume not running.
            false
        }
    }
}

// ─── Stale resource cleanup ─────────────────────────────────────────────────

/// Clean up stale resources from a crashed or improperly-stopped daemon.
/// Removes stale PID file, socket file, and any lock files if the daemon is not running.
fn cleanup_stale_resources() -> Result<()> {
    // Clean stale PID file.
    if let Some(pid) = read_pid()? {
        if !is_process_running(pid) {
            remove_pid()?;
            println!("  ⚠ cleaned stale PID file (PID {pid} not running)");
        }
    }

    // Clean stale socket file.
    if let Some(sock) = socket_path() {
        if sock.exists() {
            // Only remove if no daemon is running.
            let daemon_running = read_pid()?.map(is_process_running).unwrap_or(false);
            if !daemon_running {
                let _ = fs::remove_file(&sock);
                println!("  ⚠ cleaned stale socket file");
            }
        }
    }

    // Clean stale lock files in state dir.
    if let Ok(state) = state_dir() {
        let lock_path = state.join("daemon.lock");
        if lock_path.exists() {
            let daemon_running = read_pid()?.map(is_process_running).unwrap_or(false);
            if !daemon_running {
                let _ = fs::remove_file(&lock_path);
                println!("  ⚠ cleaned stale lock file");
            }
        }
    }

    Ok(())
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
        println!("  ⚠ cleaned stale PID file (PID {pid})");
    }

    // Clean up stale resources before starting.
    cleanup_stale_resources()?;

    // Ensure log directory exists.
    let logs = log_dir()?;
    fs::create_dir_all(&logs).with_context(|| format!("create log dir: {}", logs.display()))?;

    // Open log file for daemon output redirection.
    let log_file_path = log_path()?;
    let log_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
        .with_context(|| format!("open log file: {}", log_file_path.display()))?;
    let log_file_err = log_file
        .try_clone()
        .with_context(|| "clone log file handle for stderr")?;

    let binary = daemon_binary();
    println!("  starting daemon: {binary}");
    println!("  logs: {}", log_file_path.display());

    let mut command = Command::new(&binary);
    if let Ok(cfg) = crate::config::load() {
        command
            .env("GGNMEM_LOG_LEVEL", &cfg.daemon.log_level)
            .env(
                "GGNMEM_RETENTION_DAYS",
                cfg.retention.retention_days.to_string(),
            )
            .env(
                "GGNMEM_MAX_COMMANDS",
                cfg.retention.max_commands.to_string(),
            )
            .env(
                "GGNMEM_AUTO_CLEANUP",
                if cfg.retention.auto_cleanup {
                    "true"
                } else {
                    "false"
                },
            );
    }

    let child = command
        .stdout(log_file)
        .stderr(log_file_err)
        .stdin(std::process::Stdio::null())
        .spawn()
        .with_context(|| format!("spawn daemon: {binary}"))?;

    let pid = child.id();
    write_pid(pid)?;

    // Give the daemon a moment to start, then verify it's running.
    std::thread::sleep(std::time::Duration::from_millis(500));

    if is_process_running(pid) {
        println!("  ✓ daemon started (PID {pid})");

        // Phase 16F: Run startup health check.
        startup_health_check(pid, &log_file_path);
    } else {
        remove_pid()?;

        // Show the last few log lines to help diagnose the crash.
        let crash_log = read_last_log_lines(&log_file_path, 10);
        let mut msg = format!(
            "daemon exited immediately after start — check logs:\n  {}",
            log_file_path.display()
        );
        if !crash_log.is_empty() {
            msg.push_str("\n\nlast log output:\n");
            for line in &crash_log {
                msg.push_str("  ");
                msg.push_str(line);
                msg.push('\n');
            }
        }
        bail!("{msg}");
    }

    Ok(())
}

/// Run a startup health check on a newly-started daemon.
///
/// Verifies the daemon is responsive by waiting for the PID file advisory lock
/// and checking that the process is still alive after a brief settling period.
/// Non-fatal: prints warnings but does not fail.
fn startup_health_check(pid: u32, log_path: &std::path::Path) {
    // Wait up to 2.5 seconds for the daemon to settle.
    let mut healthy = false;
    for _ in 0..5 {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if !is_process_running(pid) {
            eprintln!("  ⚠ daemon crashed shortly after start (PID {pid})");
            let crash_log = read_last_log_lines(log_path, 5);
            if !crash_log.is_empty() {
                eprintln!("  last log output:");
                for line in &crash_log {
                    eprintln!("    {line}");
                }
            }
            eprintln!("  try: ggnmem restart");
            return;
        }
        // If still alive after 1 second, consider healthy.
        healthy = true;
    }

    if healthy {
        println!("  ✓ health check passed (daemon responsive)");
    }
}

/// Read the last N lines from a log file.
fn read_last_log_lines(path: &std::path::Path, n: usize) -> Vec<String> {
    match fs::read_to_string(path) {
        Ok(contents) => {
            let lines: Vec<&str> = contents.lines().collect();
            let start = lines.len().saturating_sub(n);
            lines[start..].iter().map(|s| s.to_string()).collect()
        }
        Err(_) => Vec::new(),
    }
}

// ─── Stop ────────────────────────────────────────────────────────────────────

pub fn cmd_stop() -> Result<()> {
    match read_pid()? {
        Some(pid) => {
            if !is_process_running(pid) {
                remove_pid()?;
                cleanup_stale_resources()?;
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

                // Clean up socket file after stopping.
                if let Some(sock) = socket_path() {
                    if sock.exists() {
                        let _ = fs::remove_file(&sock);
                    }
                }

                println!("  ✓ daemon stopped (PID {pid})");
            } else {
                println!("  ✗ failed to stop daemon (PID {pid})");
                println!("    try: kill -9 {pid}");
            }
            Ok(())
        }
        None => {
            // No PID file, but try to clean up any stale resources.
            cleanup_stale_resources()?;
            println!("  ✗ daemon not running (no PID file)");
            Ok(())
        }
    }
}

// ─── Restart ─────────────────────────────────────────────────────────────────

pub fn cmd_restart() -> Result<()> {
    cmd_stop()?;
    println!();
    // Brief pause to allow socket release.
    std::thread::sleep(std::time::Duration::from_millis(300));
    cleanup_stale_resources()?;
    cmd_start()
}

// ─── Logs ────────────────────────────────────────────────────────────────────

/// `ggnmem logs` — show daemon log output.
pub fn cmd_logs(args: &[String]) -> Result<()> {
    let lines: usize = crate::parse_named_arg(args, "--lines")
        .and_then(|v| v.parse().ok())
        .unwrap_or(50);

    let path = log_path()?;
    if !path.exists() {
        println!("no log file found at: {}", path.display());
        println!("start the daemon with: ggnmem start");
        return Ok(());
    }

    let file =
        fs::File::open(&path).with_context(|| format!("open log file: {}", path.display()))?;
    let reader = BufReader::new(file);
    let all_lines: Vec<String> = reader.lines().map_while(Result::ok).collect();

    let start = all_lines.len().saturating_sub(lines);
    let tail = &all_lines[start..];

    println!("─── {} (last {} lines) ───", path.display(), tail.len());
    println!();
    for line in tail {
        println!("{line}");
    }

    // Show log file size.
    if let Ok(meta) = fs::metadata(&path) {
        let size = meta.len();
        let size_str = if size < 1024 {
            format!("{size} B")
        } else if size < 1024 * 1024 {
            format!("{:.1} KB", size as f64 / 1024.0)
        } else {
            format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
        };
        println!();
        println!("── log file: {} ({}) ──", path.display(), size_str);
    }

    Ok(())
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
WatchdogSec=60
Environment=XDG_RUNTIME_DIR=%t
Environment=GGNMEM_LOG_LEVEL=info
Environment=GGNMEM_RETENTION_DAYS=365
Environment=GGNMEM_MAX_COMMANDS=1000000
Environment=GGNMEM_AUTO_CLEANUP=true
StandardOutput=append:%h/.local/state/ggnmem/logs/daemon.log
StandardError=append:%h/.local/state/ggnmem/logs/daemon.log

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
    // Ensure log directory exists before enabling autostart.
    let logs = log_dir()?;
    fs::create_dir_all(&logs).with_context(|| format!("create log dir: {}", logs.display()))?;

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

/// `ggnmem autostart status` — check if autostart is configured.
pub fn cmd_autostart_status() -> Result<()> {
    println!("ggnmem autostart status");
    println!("─────────────────────────────────");

    let mut found = false;

    // Check systemd.
    if has_systemd() {
        let service_path = systemd_service_path()?;
        print!("  systemd service ... ");
        if service_path.exists() {
            // Check if enabled.
            let status = Command::new("systemctl")
                .args(["--user", "is-enabled", "ggnmem-daemon.service"])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .output();
            match status {
                Ok(output) => {
                    let state = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                    println!("{state}");
                    if state == "enabled" {
                        found = true;
                    }
                }
                Err(_) => println!("unknown"),
            }

            // Check if active.
            let active = Command::new("systemctl")
                .args(["--user", "is-active", "ggnmem-daemon.service"])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .output();
            if let Ok(output) = active {
                let state = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                println!("  service active  ... {state}");
            }
        } else {
            println!("not installed");
        }
    } else {
        println!("  systemd         ... not available");
    }

    // Check shell rc fallback.
    for rc_name in &[".bashrc", ".zshrc"] {
        let rc_path = home_dir()?.join(rc_name);
        if rc_path.exists() {
            if let Ok(contents) = fs::read_to_string(&rc_path) {
                if contents.contains(AUTOSTART_MARKER) {
                    println!("  shell fallback  ... ✓ configured in {rc_name}");
                    found = true;
                }
            }
        }
    }

    if !found {
        println!();
        println!("  autostart is not configured.");
        println!("  enable with: ggnmem autostart enable");
    }

    // Show daemon status.
    println!();
    let (running, pid) = daemon_status()?;
    if running {
        if let Some(p) = pid {
            println!("  daemon          ... ✓ running (PID {p})");
        } else {
            println!("  daemon          ... ✓ running");
        }
    } else {
        println!("  daemon          ... ✗ not running");
    }

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
    println!("  \u{26a0} systemd not available, using shell startup fallback");

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
            println!("  \u{2713} autostart already configured in {}", rc_path.display());
            return Ok(());
        }
    }

    // Enhanced shell fallback with stale PID cleanup, health check, and log redirect.
    let block = format!(
        "\n{AUTOSTART_MARKER}\n\
         if [ -f \"$HOME/.local/state/ggnmem/daemon.pid\" ]; then\n  \
             _ggnmem_pid=$(cat \"$HOME/.local/state/ggnmem/daemon.pid\" 2>/dev/null)\n  \
             if [ -n \"$_ggnmem_pid\" ] && ! kill -0 \"$_ggnmem_pid\" 2>/dev/null; then\n    \
                 rm -f \"$HOME/.local/state/ggnmem/daemon.pid\" 2>/dev/null\n    \
                 rm -f \"${{XDG_RUNTIME_DIR:-/tmp}}/ggnmem/daemon.sock\" 2>/dev/null\n  \
             fi\n  \
             unset _ggnmem_pid\n\
         fi\n\
         if ! pgrep -x ggnmem-daemon > /dev/null 2>&1; then\n  \
             mkdir -p \"$HOME/.local/state/ggnmem/logs\"\n  \
             ggnmem-daemon >> \"$HOME/.local/state/ggnmem/logs/daemon.log\" 2>&1 & disown 2>/dev/null\n\
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

    println!("  \u{2713} autostart added to {}", rc_path.display());
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
