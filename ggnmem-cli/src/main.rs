mod config;
mod export;
mod hooks;
mod profile;
mod service;
mod setup;
mod tui;

use anyhow::{bail, Context, Result};
use std::path::PathBuf;
use ggnmem_daemon::{
    protocol::{
        CommandPayload, DaemonRequest, DaemonResponse, DaemonResponseKind, SessionPayload,
        PROTOCOL_VERSION,
    },
    DaemonConfig, IpcClient,
};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("ping") => ping().await,
        Some("status" | "health") => status().await,
        Some("init") => init(&args),
        Some("ingest") => ingest(&args).await,
        Some("recent") => recent().await,
        Some("count") => count().await,
        Some("doctor") => doctor().await,
        Some("search") => search(&args).await,
        Some("cleanup") => cleanup().await,
        Some("ui") => tui::run_tui().await,
        Some("version" | "--version" | "-V") => {
            version();
            Ok(())
        }
        Some("install") => setup::install(),
        Some("uninstall") => setup::uninstall(&args),
        Some("config") => cmd_config(&args),
        Some("profile") => cmd_profile(&args),
        Some("start") => service::cmd_start(),
        Some("stop") => service::cmd_stop(),
        Some("restart") => service::cmd_restart(),
        Some("autostart") => cmd_autostart(&args),
        Some("export") => export::cmd_export(&args).await,
        Some(command) => bail!("unknown command: {command}"),
        None => {
            print_usage();
            Ok(())
        }
    }
}

fn print_usage() {
    println!("ggnmem {} — semantic terminal memory engine", env!("CARGO_PKG_VERSION"));
    println!();
    println!("usage: ggnmem <command>");
    println!();
    println!("commands:");
    println!("  init <bash|zsh>  Generate shell integration script");
    println!("  ui               Interactive search interface (TUI)");
    println!("  recent           Show recent captured commands");
    println!("  search <query>   Search captured commands");
    println!("  count            Show total number of indexed commands");
    println!("  cleanup          Remove internal ggnmem commands from database");
    println!("  export           Export command history (--format json|csv)");
    println!();
    println!("daemon:");
    println!("  start            Start the daemon in background");
    println!("  stop             Stop the running daemon");
    println!("  restart          Restart the daemon");
    println!("  status           Show daemon status");
    println!("  autostart        Enable/disable daemon autostart");
    println!();
    println!("config:");
    println!("  config show      Show current configuration");
    println!("  config set K V   Set a config value");
    println!("  profile list     Show available profiles");
    println!("  profile apply N  Apply a named profile");
    println!();
    println!("setup:");
    println!("  install          Set up shell integration and config");
    println!("  uninstall        Remove ggnmem (--full to include database)");
    println!("  doctor           Check installation and daemon health");
    println!("  version          Show version");
    println!();
    println!("search options:");
    println!("  --limit N        Maximum results (default: 20)");
    println!("  --cwd            Boost results from current directory");
    println!("  --recent         Sort by recency only");
    println!("  --json           Output as JSON");
}

// ─── Subcommand routers ──────────────────────────────────────────────────────

fn cmd_config(args: &[String]) -> Result<()> {
    match args.get(2).map(String::as_str) {
        Some("show") | None => config::cmd_show(),
        Some("set") => config::cmd_set(args),
        Some(sub) => bail!("unknown config subcommand: {sub}\n\nusage:\n  ggnmem config show\n  ggnmem config set <key> <value>"),
    }
}

fn cmd_profile(args: &[String]) -> Result<()> {
    match args.get(2).map(String::as_str) {
        Some("list") | None => profile::cmd_list(),
        Some("apply") => profile::cmd_apply(args),
        Some(sub) => bail!("unknown profile subcommand: {sub}\n\nusage:\n  ggnmem profile list\n  ggnmem profile apply <name>"),
    }
}

fn cmd_autostart(args: &[String]) -> Result<()> {
    match args.get(2).map(String::as_str) {
        Some("enable") => service::cmd_autostart_enable(),
        Some("disable") => service::cmd_autostart_disable(),
        Some(sub) => bail!("unknown autostart subcommand: {sub}\n\nusage:\n  ggnmem autostart enable\n  ggnmem autostart disable"),
        None => bail!("usage:\n  ggnmem autostart enable\n  ggnmem autostart disable"),
    }
}

// ─── Version ─────────────────────────────────────────────────────────────────

fn version() {
    println!("ggnmem {}", env!("CARGO_PKG_VERSION"));
}

// ─── Existing commands ───────────────────────────────────────────────────────

async fn ping() -> Result<()> {
    let response = request(DaemonRequest::ping()).await?;
    match response.kind {
        DaemonResponseKind::Pong => {
            println!("pong");
            Ok(())
        }
        DaemonResponseKind::Error { code, message } => bail!("{code}: {message}"),
        other => bail!("unexpected daemon response: {other:?}"),
    }
}

async fn status() -> Result<()> {
    let response = request(DaemonRequest::health()).await?;
    match response.kind {
        DaemonResponseKind::Health(status) => {
            println!("state: {:?}", status.state);
            println!("uptime_ms: {}", status.uptime_ms);
            println!("queue: {}/{}", status.queue_depth, status.queue_capacity);
            println!("db_connected: {}", status.db_connected);
            println!("platform: {}", status.platform);
            Ok(())
        }
        DaemonResponseKind::Error { code, message } => bail!("{code}: {message}"),
        other => bail!("unexpected daemon response: {other:?}"),
    }
}

// ─── Shell hook generation ───────────────────────────────────────────────────

fn init(args: &[String]) -> Result<()> {
    match args.get(2).map(String::as_str) {
        Some("bash") => {
            print!("{}", hooks::bash_hook());
            Ok(())
        }
        Some("zsh") => {
            print!("{}", hooks::zsh_hook());
            Ok(())
        }
        Some(shell) => bail!("unsupported shell: {shell} (supported: bash, zsh)"),
        None => bail!("usage: ggnmem init <bash|zsh>"),
    }
}

// ─── Ingest (called by shell hooks) ──────────────────────────────────────────

async fn ingest(args: &[String]) -> Result<()> {
    let command = parse_named_arg(args, "--command").context("--command is required")?;
    let cwd = parse_named_arg(args, "--cwd").context("--cwd is required")?;
    let exit_code = parse_named_arg(args, "--exit-code").and_then(|v| v.parse::<i32>().ok());
    let duration_ms = parse_named_arg(args, "--duration-ms").and_then(|v| v.parse::<i64>().ok());
    let shell = parse_named_arg(args, "--shell");
    let session_id =
        parse_named_arg(args, "--session-id").unwrap_or_else(|| format!("{}", std::process::id()));
    let hostname = parse_named_arg(args, "--hostname").unwrap_or_else(|| "unknown".to_owned());
    let started_at_ms =
        parse_named_arg(args, "--started-at-ms").and_then(|v| v.parse::<i64>().ok());
    let completed_at_ms = parse_named_arg(args, "--completed-at-ms")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0)
        });

    let command_id = uuid::Uuid::new_v4().to_string();

    let session = SessionPayload {
        session_id: session_id.clone(),
        os_context: "linux".to_owned(),
        hostname,
        shell,
        started_at_ms: started_at_ms.unwrap_or(completed_at_ms),
    };

    let command_payload = CommandPayload {
        command_id,
        session_id,
        command,
        cwd,
        exit_code,
        duration_ms,
        started_at_ms,
        completed_at_ms,
    };

    let ingest_request = DaemonRequest::IngestCommand {
        version: PROTOCOL_VERSION,
        session: Box::new(session),
        command: Box::new(command_payload),
    };

    // Best-effort: if daemon is unavailable, silently exit.
    // The hook runs in background, so no user-visible error is needed.
    let _ = request(ingest_request).await;
    Ok(())
}

/// Parse a `--name value` pair from the argument list.
fn parse_named_arg(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

/// Check if a boolean flag is present in the argument list.
fn has_flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

// ─── Recent commands ─────────────────────────────────────────────────────────

async fn recent() -> Result<()> {
    let response = request(DaemonRequest::query_recent(20)).await?;
    match response.kind {
        DaemonResponseKind::RecentCommands { commands } => {
            if commands.is_empty() {
                println!("no commands captured yet");
                return Ok(());
            }
            for cmd in &commands {
                let exit_str = cmd
                    .exit_code
                    .map(|c| format!("[{c}]"))
                    .unwrap_or_else(|| "[?]".to_owned());
                let duration_str = cmd
                    .duration_ms
                    .map(|d| format!("{d}ms"))
                    .unwrap_or_else(|| "?".to_owned());
                let ts = format_timestamp(cmd.completed_at_ms);
                println!(
                    "{exit_str:>5} {duration_str:>8}  {ts}  {dir}  {cmd}",
                    dir = cmd.cwd,
                    cmd = cmd.command,
                );
            }
            Ok(())
        }
        DaemonResponseKind::Error { code, message } => bail!("{code}: {message}"),
        other => bail!("unexpected daemon response: {other:?}"),
    }
}

fn format_timestamp(millis: i64) -> String {
    use std::time::{Duration, UNIX_EPOCH};

    let secs = (millis / 1000) as u64;
    let system_time = UNIX_EPOCH + Duration::from_secs(secs);
    let datetime: std::time::SystemTime = system_time;

    // Format as a human-readable local time.
    // Since we don't have chrono, use a simple approach.
    let duration_since_epoch = datetime.duration_since(UNIX_EPOCH).unwrap_or_default();
    let total_secs = duration_since_epoch.as_secs();

    // Simple UTC formatting without external dependency.
    let secs_in_day = total_secs % 86400;
    let hours = secs_in_day / 3600;
    let minutes = (secs_in_day % 3600) / 60;
    let seconds = secs_in_day % 60;

    let days = total_secs / 86400;
    // Simple date calculation from epoch days.
    let (year, month, day) = epoch_days_to_date(days);

    format!("{year:04}-{month:02}-{day:02} {hours:02}:{minutes:02}:{seconds:02}")
}

fn epoch_days_to_date(days: u64) -> (u64, u64, u64) {
    // Algorithm from Howard Hinnant's chrono-compatible date library.
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

fn format_duration(millis: Option<i64>) -> String {
    match millis {
        Some(ms) if ms < 1000 => format!("{ms}ms"),
        Some(ms) if ms < 60_000 => format!("{:.1}s", ms as f64 / 1000.0),
        Some(ms) => format!("{:.1}m", ms as f64 / 60_000.0),
        None => "—".to_owned(),
    }
}

fn format_match_kind(kind: &ggnmem_db::MatchKind) -> &'static str {
    use ggnmem_db::MatchKind;
    match kind {
        MatchKind::Exact => "exact",
        MatchKind::Prefix => "prefix",
        MatchKind::Partial => "partial",
        MatchKind::Fuzzy => "fuzzy",
    }
}

// ─── Count ───────────────────────────────────────────────────────────────────

async fn count() -> Result<()> {
    let response = request(DaemonRequest::count_commands()).await?;
    match response.kind {
        DaemonResponseKind::CommandCount { count } => {
            println!("{count}");
            Ok(())
        }
        DaemonResponseKind::Error { code, message } => bail!("{code}: {message}"),
        other => bail!("unexpected daemon response: {other:?}"),
    }
}

// ─── Cleanup ─────────────────────────────────────────────────────────────────

async fn cleanup() -> Result<()> {
    let response = request(DaemonRequest::cleanup_commands()).await?;
    match response.kind {
        DaemonResponseKind::CleanupResult { removed, remaining } => {
            if removed == 0 {
                println!("No internal commands found. Database is clean.");
            } else {
                println!("Removed {removed} internal commands.");
            }
            println!("Database optimized. {remaining} commands remaining.");
            Ok(())
        }
        DaemonResponseKind::Error { code, message } => bail!("{code}: {message}"),
        other => bail!("unexpected daemon response: {other:?}"),
    }
}

// ─── Doctor ──────────────────────────────────────────────────────────────────

async fn doctor() -> Result<()> {
    println!("ggnmem doctor");
    println!("─────────────────────────────────");
    println!();

    // ── Offline checks (no daemon required) ──

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("~"));

    // Version.
    println!("version         ... {}", env!("CARGO_PKG_VERSION"));

    // Binary install.
    let bin_dir = home.join(".local").join("bin");
    let cli_bin = bin_dir.join("ggnmem");
    let daemon_bin = bin_dir.join("ggnmem-daemon");
    print!("ggnmem binary   ... ");
    if cli_bin.exists() {
        println!("✓ {}", cli_bin.display());
    } else {
        println!("✗ not found at {}", cli_bin.display());
    }
    print!("daemon binary   ... ");
    if daemon_bin.exists() {
        println!("✓ {}", daemon_bin.display());
    } else {
        println!("✗ not found at {}", daemon_bin.display());
    }

    // Config.
    let config_file = home.join(".config").join("ggnmem").join("config.toml");
    print!("config          ... ");
    if config_file.exists() {
        println!("✓ {}", config_file.display());
    } else {
        println!("✗ not found (run: ggnmem install)");
    }

    // Config details (features + profile).
    match config::load() {
        Ok(cfg) => {
            let profile_name = profile::detect_profile(&cfg)
                .unwrap_or("custom");
            println!("  profile       ... {profile_name}");
            println!(
                "  features      ... capture={} search={} tui={} ai={}",
                cfg.features.capture,
                cfg.features.search,
                cfg.features.tui,
                cfg.features.ai
            );
            println!("  max_history   ... {}", cfg.limits.max_history);
            println!("  index_mode    ... {}", cfg.search.index_mode);
        }
        Err(_) => {
            println!("  config        ... (could not load)");
        }
    }

    // Database.
    let db_path = home.join(".local").join("share").join("ggnmem").join("ggnmem.db");
    print!("database        ... ");
    if db_path.exists() {
        let size = std::fs::metadata(&db_path)
            .map(|m| m.len())
            .unwrap_or(0);
        let size_str = if size < 1024 {
            format!("{size} B")
        } else if size < 1024 * 1024 {
            format!("{:.1} KB", size as f64 / 1024.0)
        } else {
            format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
        };
        println!("✓ {} ({})", db_path.display(), size_str);
    } else {
        println!("✗ not found (start daemon to create)");
    }

    // Shell integration.
    print!("shell hooks     ... ");
    let bashrc = home.join(".bashrc");
    let zshrc = home.join(".zshrc");
    let mut shell_found = false;
    if zshrc.exists() {
        if let Ok(contents) = std::fs::read_to_string(&zshrc) {
            if contents.contains("ggnmem init") {
                print!("✓ zsh ");
                shell_found = true;
            }
        }
    }
    if bashrc.exists() {
        if let Ok(contents) = std::fs::read_to_string(&bashrc) {
            if contents.contains("ggnmem init") {
                print!("✓ bash ");
                shell_found = true;
            }
        }
    }
    if shell_found {
        println!();
    } else {
        println!("✗ not configured (run: ggnmem install)");
    }

    // ── Online checks (daemon required) ──

    println!();
    print!("daemon          ... ");

    // First check PID file.
    let (pid_running, pid_val) = service::daemon_status()?;
    let health_result = request(DaemonRequest::health()).await;
    match &health_result {
        Ok(response) => match &response.kind {
            DaemonResponseKind::Health(status) => {
                if let Some(pid) = pid_val {
                    println!("✓ running (PID {pid})");
                } else {
                    println!("✓ running");
                }
                println!("  state         ... {:?}", status.state);
                println!("  uptime        ... {}ms", status.uptime_ms);
                println!(
                    "  queue         ... {}/{}",
                    status.queue_depth, status.queue_capacity
                );
                println!(
                    "  db connected  ... {}",
                    if status.db_connected {
                        "✓"
                    } else {
                        "✗"
                    }
                );
                println!("  platform      ... {}", status.platform);
            }
            DaemonResponseKind::Error { code, message } => {
                println!("✗ error: {code}: {message}");
            }
            other => {
                println!("✗ unexpected: {other:?}");
            }
        },
        Err(_error) => {
            if pid_running {
                println!("✗ PID file exists but IPC failed");
            } else {
                println!("✗ not running");
            }
            println!("  start with: ggnmem start");
        }
    }

    // Capture check.
    print!("capture         ... ");
    match config::load() {
        Ok(cfg) if cfg.features.capture => println!("✓ enabled"),
        Ok(_) => println!("✗ disabled (ggnmem config set capture true)"),
        Err(_) => println!("? (config not loaded)"),
    }

    // Command count (only if daemon is reachable).
    if health_result.is_ok() {
        print!("commands        ... ");
        match request(DaemonRequest::count_commands()).await {
            Ok(response) => match response.kind {
                DaemonResponseKind::CommandCount { count } => {
                    println!("{count} indexed");
                }
                DaemonResponseKind::Error { code, message } => {
                    println!("error: {code}: {message}");
                }
                _ => println!("unexpected response"),
            },
            Err(error) => println!("error: {error}"),
        }
    }

    println!();
    println!("─────────────────────────────────");
    println!("all checks complete");
    Ok(())
}

// ─── Search ──────────────────────────────────────────────────────────────────

async fn search(args: &[String]) -> Result<()> {
    let limit = parse_named_arg(args, "--limit")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(20);
    let json_output = has_flag(args, "--json");
    let recent_only = has_flag(args, "--recent");
    let use_cwd = has_flag(args, "--cwd");

    // Resolve current working directory for --cwd boosting.
    let cwd = if use_cwd {
        std::env::current_dir()
            .ok()
            .and_then(|p| p.to_str().map(String::from))
    } else {
        None
    };

    // Build query from positional args (skip "ggnmem", "search", and any --flag/value pairs).
    let valued_flags = ["--limit"];
    let boolean_flags = ["--json", "--cwd", "--recent"];

    let mut query_parts: Vec<&str> = Vec::new();
    let mut skip_next = false;
    for arg in args.iter().skip(2) {
        if skip_next {
            skip_next = false;
            continue;
        }
        if valued_flags.contains(&arg.as_str()) {
            skip_next = true;
            continue;
        }
        if boolean_flags.contains(&arg.as_str()) {
            continue;
        }
        query_parts.push(arg);
    }

    let query = query_parts.join(" ");
    if query.is_empty() {
        bail!("usage: ggnmem search <query> [--limit N] [--cwd] [--recent] [--json]");
    }

    let response = request(DaemonRequest::search_commands_with_options(
        &query,
        limit,
        cwd,
        recent_only,
    ))
    .await?;

    match response.kind {
        DaemonResponseKind::SearchResults { results } => {
            if results.is_empty() {
                println!("no matching commands found for: {query}");
                return Ok(());
            }

            if json_output {
                let json = serde_json::to_string_pretty(&results)
                    .context("serialize search results to JSON")?;
                println!("{json}");
                return Ok(());
            }

            println!(
                "found {} result{} for: {query}",
                results.len(),
                if results.len() == 1 { "" } else { "s" }
            );
            println!();

            for result in &results {
                let exit_str = result
                    .exit_code
                    .map(|c| {
                        if c == 0 {
                            "  ✓ ".to_owned()
                        } else {
                            format!("✗{c:>2} ")
                        }
                    })
                    .unwrap_or_else(|| " ?  ".to_owned());
                let ts = format_timestamp(result.completed_at_ms);
                let dur = format_duration(result.duration_ms);
                let match_tag = format_match_kind(&result.match_kind);
                let score_pct = (result.score * 100.0) as u32;

                println!(
                    "  {exit_str} {ts}  {dur:>7}  [{match_tag:>7} {score_pct:>3}%]  {cwd}",
                    cwd = result.cwd
                );
                println!("       $ {cmd}", cmd = result.command);
                if result.run_count > 1 {
                    println!("         (run {} times)", result.run_count);
                }
                println!();
            }

            Ok(())
        }
        DaemonResponseKind::Error { code, message } => bail!("{code}: {message}"),
        other => bail!("unexpected daemon response: {other:?}"),
    }
}

// ─── IPC helper ──────────────────────────────────────────────────────────────

async fn request(request: DaemonRequest) -> Result<DaemonResponse> {
    let config = DaemonConfig::load().context("load daemon client configuration")?;
    let mut client = IpcClient::connect(&config.endpoint)
        .await
        .context("connect to ggnmem daemon")?;
    client
        .request(&request)
        .await
        .context("daemon request failed")
}
