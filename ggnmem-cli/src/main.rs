mod config;
mod export;
mod hooks;
mod import;
mod profile;
mod service;
mod setup;
mod tui;
mod update;
mod upgrade;

use anyhow::{bail, Context, Result};
use ggnmem_daemon::{
    protocol::{
        CommandPayload, DaemonRequest, DaemonResponse, DaemonResponseKind, SessionPayload,
        PROTOCOL_VERSION,
    },
    DaemonConfig, IpcClient,
};
use std::path::PathBuf;

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
        Some("cleanup") => cleanup(&args).await,
        Some("optimize") => optimize().await,
        Some("db") => db(&args).await,
        Some("stats") => stats().await,
        Some("ui") => tui::run_tui().await,
        Some("version" | "--version" | "-V") => {
            version(&args);
            Ok(())
        }
        Some("install") => setup::install(),
        Some("uninstall") => setup::uninstall(&args),
        Some("config") => cmd_config(&args),
        Some("profile") => cmd_profile(&args),
        Some("start") => service::cmd_start(),
        Some("stop") => service::cmd_stop(),
        Some("restart") => service::cmd_restart(),
        Some("logs") => service::cmd_logs(&args),
        Some("autostart") => cmd_autostart(&args),
        Some("export") => export::cmd_export(&args).await,
        Some("ai") => cmd_ai(&args),
        Some("semantic") => semantic(&args).await,
        Some("update") => update::cmd_update(&args),
        Some("self-update") => update::cmd_self_update(&args),
        Some("upgrade") => upgrade::cmd_upgrade(&args),
        Some("ask") => cmd_ask(&args),
        Some("explain") => cmd_explain(&args),
        Some("learn") => cmd_learn(&args),
        Some("knowledge") => cmd_knowledge(&args),
        Some("import") => import::cmd_import(&args),
        Some(command) => bail!("unknown command: {command}"),
        None => {
            print_usage();
            Ok(())
        }
    }
}

fn print_usage() {
    println!(
        "ggnmem {} — semantic terminal memory engine",
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("usage: ggnmem <command>");
    println!();
    println!("commands:");
    println!("  init <bash|zsh>  Generate shell integration script");
    println!("  ui               Interactive search interface (TUI)");
    println!("  recent           Show recent captured commands");
    println!("  search <query>   Search captured commands");
    println!("  count            Show total number of indexed commands");
    println!("  stats            Show detailed database usage statistics");
    println!("  optimize         Run database optimization (defragment and analyze)");
    println!("  db stats         Show low-level database statistics");
    println!("  cleanup [flag]   Remove commands (--internal, --duplicates, --failed, --older-than DAYS)");
    println!("  export           Export command history (--format json|csv)");
    println!();
    println!("knowledge base:");
    println!("  ask <query>      Get command suggestions from knowledge base");
    println!("  explain <cmd>    Explain a command (purpose, flags, examples)");
    println!(
        "  learn <topic>    Learn commands for a topic (docker, git, linux, cargo, go, kubernetes)"
    );
    println!("  knowledge list   List all loaded knowledge packs");
    println!("  knowledge validate Validate custom knowledge packs");
    println!();
    println!("daemon:");
    println!("  start            Start the daemon in background");
    println!("  stop             Stop the running daemon");
    println!("  restart          Restart the daemon");
    println!("  status           Show daemon status");
    println!("  logs             Show daemon logs (--lines N)");
    println!("  autostart        Enable/disable/status daemon autostart");
    println!();
    println!("config:");
    println!("  config show      Show current configuration");
    println!("  config set K V   Set a config value");
    println!("  profile list     Show available profiles");
    println!("  profile apply N  Apply a named profile");
    println!();
    println!("ai:");
    println!("  ai status        Show AI feature status");
    println!("  ai enable        Enable AI features");
    println!("  ai disable       Disable AI features");
    println!("  ai models        List available/installed models");
    println!("  ai install [M]   Install an embedding model (interactive if no model given)");
    println!("  ai remove M      Remove an installed model");
    println!("  ai use M         Switch the active embedding model");
    println!("  ai benchmark     Compare installed models performance");
    println!("  ai setup         Guided AI setup wizard");
    println!("  ai doctor        Run AI diagnostics");
    println!("  ai verify-model  Verify model loads and produces embeddings");
    println!("  ai reindex       Rebuild all embeddings");
    println!();
    println!("history import:");
    println!("  import auto      Auto-detect shell and import history");
    println!("  import bash      Import from ~/.bash_history");
    println!("  import zsh       Import from ~/.zsh_history");
    println!("  import fish      Import from ~/.local/share/fish/fish_history");
    println!("  --dry-run        Show counts without modifying the database");
    println!("  --preview        Show a sample of commands before importing");
    println!();
    println!("setup:");
    println!("  install          Set up shell integration and config");
    println!("  uninstall        Remove ggnmem (--full to include database)");
    println!("  update           Check for updates");
    println!("  self-update      Update ggnmem to the latest version");
    println!("  upgrade          Upgrade from a local release bundle");
    println!("  doctor           Check installation and daemon health");
    println!("  version          Show version (--verbose for extended info)");
    println!();
    println!("search options:");
    println!("  --limit N        Maximum results (default: 20)");
    println!("  --cwd            Boost results from current directory");
    println!("  --recent         Sort by recency only");
    println!("  --mode MODE      Search mode: fts, semantic, hybrid (default: hybrid)");
    println!("  --json           Output as JSON");
    println!("  --debug          Show source breakdown and latency");
    println!();
    println!("semantic search:");
    println!("  semantic <query>  Semantic search (vector similarity)");
    println!("  --limit N        Maximum results (default: 10)");
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
        Some("status") => service::cmd_autostart_status(),
        Some(sub) => bail!("unknown autostart subcommand: {sub}\n\nusage:\n  ggnmem autostart enable\n  ggnmem autostart disable\n  ggnmem autostart status"),
        None => service::cmd_autostart_status(),
    }
}

fn cmd_ai(args: &[String]) -> Result<()> {
    match args.get(2).map(String::as_str) {
        Some("status") | None => ai_status(),
        Some("enable") => ai_enable(args),
        Some("disable") => ai_disable(),
        Some("models") => ai_models(),
        Some("install") => ai_install(args),
        Some("remove") => ai_remove(args),
        Some("use") => ai_use(args),
        Some("benchmark") => ai_benchmark(),
        Some("setup") => ai_setup(),
        Some("doctor") => ai_doctor(),
        Some("verify-model") => ai_verify_model(args),
        Some("reindex") => ai_reindex(),
        Some(sub) => bail!("unknown ai subcommand: {sub}\n\nusage:\n  ggnmem ai status\n  ggnmem ai enable\n  ggnmem ai disable\n  ggnmem ai models\n  ggnmem ai install [model]\n  ggnmem ai remove <model>\n  ggnmem ai use <model>\n  ggnmem ai benchmark\n  ggnmem ai setup\n  ggnmem ai doctor\n  ggnmem ai verify-model\n  ggnmem ai reindex"),
    }
}

// ─── Version ─────────────────────────────────────────────────────────────────

fn version(args: &[String]) {
    let verbose = has_flag(args, "--verbose") || has_flag(args, "-v");

    let version = env!("CARGO_PKG_VERSION");
    let build_date = env!("GGNMEM_BUILD_DATE");
    let git_commit = env!("GGNMEM_GIT_COMMIT");
    let build_profile = env!("GGNMEM_BUILD_PROFILE");
    let rustc_version = env!("GGNMEM_RUSTC_VERSION");
    let platform = env!("GGNMEM_TARGET_PLATFORM");

    // AI enabled — read from config at runtime.
    let ai_enabled = config::load().map(|cfg| cfg.ai.ai_enabled).unwrap_or(false);

    // ONNX enabled — compile-time feature check (from ggnmem-ai crate).
    let onnx_enabled = ggnmem_ai::ONNX_ENABLED;

    println!("ggnmem {version}");
    println!();
    println!("  Version:  {version}");
    println!("  Commit:   {git_commit}");
    println!("  Build:    {build_date}");
    println!("  Rust:     {rustc_version}");
    println!("  Platform: {platform}");
    println!(
        "  ONNX:     {}",
        if onnx_enabled { "enabled" } else { "disabled" }
    );
    println!(
        "  AI:       {}",
        if ai_enabled { "enabled" } else { "disabled" }
    );

    if verbose {
        println!();
        println!("  ─── verbose ───");
        println!("  Profile:  {build_profile}");
        println!("  Target:   {}", std::env::consts::ARCH);
        println!("  OS:       {}", std::env::consts::OS);
        println!("  Family:   {}", std::env::consts::FAMILY);
        println!(
            "  Binary:   {}",
            std::env::current_exe()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "unknown".to_owned())
        );

        // Config file.
        match config::config_path() {
            Ok(path) => {
                if path.exists() {
                    println!("  Config:   {}", path.display());
                } else {
                    println!("  Config:   {} (defaults)", path.display());
                }
            }
            Err(_) => println!("  Config:   unavailable"),
        }

        // Database path.
        let db_path = default_db_path();
        if db_path.exists() {
            let size = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);
            println!("  Database: {} ({})", db_path.display(), format_bytes(size));
        } else {
            println!("  Database: {} (not created)", db_path.display());
        }

        // ONNX model info (compile-time check only, no runtime load).
        if onnx_enabled {
            match config::load() {
                Ok(cfg) => {
                    let ai_cfg = build_ai_config(&cfg);
                    let mgr = ggnmem_ai::ModelManager::new(ai_cfg.models_dir);
                    let installed = mgr.is_installed(&cfg.ai.model_name);
                    println!(
                        "  Model:    {} ({})",
                        cfg.ai.model_name,
                        if installed {
                            "installed"
                        } else {
                            "not installed"
                        }
                    );
                }
                Err(_) => println!("  Model:    unknown"),
            }
        }

        // Daemon status.
        match service::daemon_status() {
            Ok((running, pid)) => {
                if running {
                    if let Some(p) = pid {
                        println!("  Daemon:   running (PID {p})");
                    } else {
                        println!("  Daemon:   running");
                    }
                } else {
                    println!("  Daemon:   not running");
                }
            }
            Err(_) => println!("  Daemon:   unknown"),
        }
    }
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
pub fn parse_named_arg(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

/// Check if a boolean flag is present in the argument list.
pub fn has_flag(args: &[String], name: &str) -> bool {
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

    // Config details (features + profile + limits).
    match config::load() {
        Ok(cfg) => {
            let profile_name = profile::detect_profile(&cfg).unwrap_or("custom");
            println!("  profile       ... {profile_name}");
            println!(
                "  features      ... capture={} search={} tui={} ai={}",
                cfg.features.capture, cfg.features.search, cfg.features.tui, cfg.features.ai
            );
            println!("  max_history   ... {}", cfg.limits.max_history);
            println!("  index_mode    ... {}", cfg.search.index_mode);
            println!("  log_level     ... {}", cfg.daemon.log_level);
            println!("  max_memory_mb ... {} MB", cfg.limits.max_memory_mb);
            println!("  max_db_size_mb ... {} MB", cfg.limits.max_db_size_mb);
            println!(
                "  retention     ... {} days, max {} commands, auto_cleanup={}",
                cfg.retention.retention_days,
                cfg.retention.max_commands,
                cfg.retention.auto_cleanup
            );
        }
        Err(_) => {
            println!("  config        ... (could not load)");
        }
    }

    // Database.
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local").join("share"));
    let db_path = data_home.join("ggnmem").join("ggnmem.db");
    print!("database        ... ");
    if db_path.exists() {
        let size = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);
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

    // Log file.
    let log_file = home
        .join(".local")
        .join("state")
        .join("ggnmem")
        .join("logs")
        .join("daemon.log");
    print!("log file        ... ");
    if log_file.exists() {
        let size = std::fs::metadata(&log_file).map(|m| m.len()).unwrap_or(0);
        let size_str = if size < 1024 {
            format!("{size} B")
        } else if size < 1024 * 1024 {
            format!("{:.1} KB", size as f64 / 1024.0)
        } else {
            format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
        };
        println!("✓ {} ({})", log_file.display(), size_str);
    } else {
        println!("— no logs yet");
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
                    if status.db_connected { "✓" } else { "✗" }
                );
                println!("  platform      ... {}", status.platform);

                // RAM usage from /proc/<pid>/status.
                if let Some(pid) = pid_val {
                    let proc_status = format!("/proc/{pid}/status");
                    if let Ok(contents) = std::fs::read_to_string(&proc_status) {
                        for line in contents.lines() {
                            if line.starts_with("VmRSS:") {
                                let rss = line.trim_start_matches("VmRSS:").trim();
                                println!("  memory (RSS)  ... {rss}");
                                break;
                            }
                        }
                    }
                }
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

        print!("db stats        ... ");
        match request(DaemonRequest::get_db_stats()).await {
            Ok(response) => match response.kind {
                DaemonResponseKind::DbStatsResult { stats } => {
                    println!(
                        "{}; {} free pages; {} duplicate runs",
                        format_bytes(stats.db_size_bytes),
                        stats.freelist_count,
                        stats.duplicate_count_estimate
                    );
                }
                DaemonResponseKind::Error { code, message } => {
                    println!("error: {code}: {message}");
                }
                _ => println!("unexpected response"),
            },
            Err(error) => println!("error: {error}"),
        }
    }

    // Autostart status.
    print!("autostart       ... ");
    let mut autostart_found = false;
    // Check systemd.
    let systemd_path = home
        .join(".config")
        .join("systemd")
        .join("user")
        .join("ggnmem-daemon.service");
    if systemd_path.exists() {
        print!("✓ systemd ");
        autostart_found = true;
    }
    // Check shell rc.
    for rc_name in &[".bashrc", ".zshrc"] {
        let rc_path = home.join(rc_name);
        if rc_path.exists() {
            if let Ok(contents) = std::fs::read_to_string(&rc_path) {
                if contents.contains("# ggnmem daemon autostart") {
                    print!("✓ shell ");
                    autostart_found = true;
                }
            }
        }
    }
    if autostart_found {
        println!();
    } else {
        println!("\u{2717} not configured (ggnmem autostart enable)");
    }

    // ── AI status (offline) ──

    println!();
    print!("ai              ... ");
    let mut ai_ok = false;
    let mut model_ok = false;
    let mut vector_db_ok = false;
    let mut vector_count: usize = 0;
    match config::load() {
        Ok(cfg) => {
            if cfg.ai.ai_enabled {
                println!("\u{2713} enabled");
                ai_ok = true;
            } else {
                println!("\u{2717} disabled");
            }
            println!("  semantic_search ... {}", cfg.ai.semantic_search);
            println!("  provider      ... {}", cfg.ai.embedding_provider);
            println!("  model         ... {}", cfg.ai.model_name);

            let ai_cfg = build_ai_config(&cfg);
            let mgr = ggnmem_ai::ModelManager::new(ai_cfg.models_dir.clone());
            let model_installed = mgr.is_installed(&cfg.ai.model_name);
            print!("  model installed ... ");
            if model_installed {
                let size_str = mgr
                    .model_size(&cfg.ai.model_name)
                    .map(format_bytes)
                    .unwrap_or_else(|| "unknown".to_owned());
                println!("\u{2713} ({})", size_str);
                model_ok = true;
            } else {
                println!("\u{2717}");
            }

            let store = ggnmem_ai::VectorStore::new(ai_cfg.vector_db_path.clone());
            print!("  vector db     ... ");
            if store.is_initialized() {
                let count = store.count().unwrap_or(0);
                vector_count = count as usize;
                println!("\u{2713} initialized ({count} vectors)");
                vector_db_ok = true;
            } else {
                println!("\u{2014} not initialized");
            }

            // AI model health: can it produce embeddings?
            print!("  model health  ... ");
            if model_ok {
                let (provider, provider_name) =
                    ggnmem_ai::create_provider(&ai_cfg.models_dir, &cfg.ai.model_name);
                match provider.embed_query("test") {
                    Ok(embedding) if !embedding.is_empty() => {
                        println!("\u{2713} ok ({provider_name}, {}d)", embedding.len());
                    }
                    Ok(_) => println!("\u{2717} produced empty embedding"),
                    Err(e) => println!("\u{2717} {e}"),
                }
            } else {
                println!("\u{2014} model not installed");
            }
        }
        Err(_) => {
            println!("? (config not loaded)");
        }
    }

    // ── Search backend status ──

    println!();
    print!("search backends ... ");
    let fts_ok = db_path.exists(); // FTS5 is always available when DB exists.
    let semantic_ok = ai_ok && model_ok && vector_db_ok && vector_count > 0;
    let hybrid_ok = fts_ok && semantic_ok;

    if fts_ok {
        print!("\u{2713} FTS5 ");
    } else {
        print!("\u{2717} FTS5 ");
    }
    if semantic_ok {
        print!("\u{2713} semantic ");
    } else {
        print!("\u{2717} semantic ");
    }
    println!();

    // Hybrid search status.
    print!("hybrid search   ... ");
    if hybrid_ok {
        println!("\u{2713} available (FTS + semantic)");
    } else if fts_ok {
        println!("\u{2714} FTS only (enable AI for hybrid)");
    } else {
        println!("\u{2717} not available (start daemon to create database)");
    }

    // ── History import status ──

    import::doctor_history_status();

    // ── Ctrl+R integration status ──

    print!("ctrl+r          ... ");
    let tui_enabled = config::load().map(|c| c.features.tui).unwrap_or(true);
    if shell_found && tui_enabled {
        println!("\u{2713} ready (shell hooks + TUI enabled)");
    } else if shell_found && !tui_enabled {
        println!("\u{2717} TUI disabled (ggnmem config set tui true)");
    } else if !shell_found && tui_enabled {
        println!("\u{2717} shell hooks not configured (run: ggnmem install)");
    } else {
        println!("\u{2717} needs shell hooks + TUI enabled");
    }

    // ── TUI diagnostics ──

    println!();
    println!("TUI             ... \u{2713} available");

    // Clipboard support detection.
    print!("  clipboard     ... ");
    let clipboard_tools: &[(&str, &[&str])] = &[
        ("clip.exe", &[]),
        ("xclip", &["-version"]),
        ("xsel", &["--version"]),
        ("wl-copy", &["--version"]),
    ];
    let mut clipboard_found = false;
    for (tool, _args) in clipboard_tools {
        if std::process::Command::new(tool)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .stdin(std::process::Stdio::null())
            .spawn()
            .and_then(|mut c| c.wait())
            .is_ok()
        {
            println!("\u{2713} {tool}");
            clipboard_found = true;
            break;
        }
    }
    if !clipboard_found {
        println!("\u{2717} no clipboard tool found (install xclip or xsel)");
    }

    println!();
    println!("\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}");
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
    let debug = has_flag(args, "--debug");
    let search_mode = match parse_named_arg(args, "--mode").as_deref() {
        Some("fts") => ggnmem_daemon::SearchMode::FtsOnly,
        Some("semantic" | "sem") => ggnmem_daemon::SearchMode::SemanticOnly,
        Some("hybrid" | "hyb") | None => ggnmem_daemon::SearchMode::Hybrid,
        Some(other) => bail!("unknown search mode: {other} (use: fts, semantic, hybrid)"),
    };

    // Resolve current working directory for --cwd boosting.
    let cwd = if use_cwd {
        std::env::current_dir()
            .ok()
            .and_then(|p| p.to_str().map(String::from))
    } else {
        None
    };

    // Build query from positional args (skip "ggnmem", "search", and any --flag/value pairs).
    let valued_flags = ["--limit", "--mode"];
    let boolean_flags = ["--json", "--cwd", "--recent", "--debug"];

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
        bail!("usage: ggnmem search <query> [--limit N] [--cwd] [--recent] [--mode fts|semantic|hybrid] [--json] [--debug]");
    }

    // Unified search: use the specified search mode.
    let start = std::time::Instant::now();
    let response = request(DaemonRequest::search_commands_with_mode(
        &query,
        limit,
        cwd,
        recent_only,
        search_mode,
    ))
    .await?;
    let elapsed_ms = start.elapsed().as_millis();

    match response.kind {
        DaemonResponseKind::SearchResults { results, .. } => {
            if results.is_empty() {
                println!("no matching commands found for: {query}");

                // ── Knowledge Base fallback ──
                let kb = ggnmem_knowledge::KnowledgeBase::new();
                let suggestions = kb.ask(&query, 3);
                if !suggestions.is_empty() {
                    println!();
                    println!("  ─── Knowledge Base Suggestions ───");
                    println!();
                    for s in &suggestions {
                        let conf = ggnmem_knowledge::format_confidence(s.confidence);
                        println!("  Suggested:    {}", s.command);
                        println!("  Description:  {}", s.description);
                        println!("  Category:     {} / {}", s.topic, s.category);
                        println!("  Confidence:   {conf}");
                        println!("  Source:        {}", s.source);
                        println!();
                    }
                }

                return Ok(());
            }

            if json_output {
                let json = serde_json::to_string_pretty(&results)
                    .context("serialize search results to JSON")?;
                println!("{json}");
                return Ok(());
            }

            // Count results by source for debug header.
            if debug {
                let fts_count = results
                    .iter()
                    .filter(|r| r.source == ggnmem_daemon::SearchSource::Fts)
                    .count();
                let sem_count = results
                    .iter()
                    .filter(|r| r.source == ggnmem_daemon::SearchSource::Semantic)
                    .count();
                let hyb_count = results
                    .iter()
                    .filter(|r| r.source == ggnmem_daemon::SearchSource::Hybrid)
                    .count();
                println!(
                    "found {} result{} for: {query}  (FTS:{fts_count} SEM:{sem_count} HYB:{hyb_count}  {elapsed_ms}ms)",
                    results.len(),
                    if results.len() == 1 { "" } else { "s" }
                );
            } else {
                println!(
                    "found {} result{} for: {query}",
                    results.len(),
                    if results.len() == 1 { "" } else { "s" }
                );
            }
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

                let source_tag = if debug {
                    format!(" [{}]", result.source)
                } else {
                    String::new()
                };

                println!(
                    "  {exit_str} {ts}  {dur:>7}  [{match_tag:>7} {score_pct:>3}%]{source_tag}  {cwd}",
                    cwd = result.cwd
                );
                println!("       $ {cmd}", cmd = result.command);
                if result.run_count > 1 {
                    println!("         (run {} times)", result.run_count);
                }
                println!();
            }

            // ── Knowledge Base supplement ──
            // Show KB suggestions when all results are low-quality partial
            // matches (no exact or FTS match), or when there are very few results.
            // This makes the KB fallback actually useful since FTS partial
            // matching almost always returns _something_.
            if !json_output {
                let has_strong_match = results.iter().any(|r| {
                    matches!(
                        r.match_kind,
                        ggnmem_db::MatchKind::Exact | ggnmem_db::MatchKind::Prefix
                    )
                });
                let best_score = results.first().map(|r| r.score).unwrap_or(0.0);

                if !has_strong_match || best_score < 0.5 {
                    let kb = ggnmem_knowledge::KnowledgeBase::new();
                    let suggestions = kb.ask(&query, 3);
                    // Only show if the KB has confident matches.
                    let good_suggestions: Vec<_> = suggestions
                        .into_iter()
                        .filter(|s| s.confidence >= 0.5)
                        .collect();
                    if !good_suggestions.is_empty() {
                        println!("  ─── Knowledge Base Suggestions ───");
                        println!();
                        for s in &good_suggestions {
                            let conf = ggnmem_knowledge::format_confidence(s.confidence);
                            println!("    $ {:<30} {} ({})", s.command, s.description, conf);
                        }
                        println!();
                    }
                }
            }

            Ok(())
        }
        DaemonResponseKind::Error { code, message } => bail!("{code}: {message}"),
        other => bail!("unexpected daemon response: {other:?}"),
    }
}

// ─── Phase 11: Cleanup, Optimize, Stats ──────────────────────────────────────

async fn cleanup(args: &[String]) -> Result<()> {
    let mode_arg = args.get(2).map(String::as_str).unwrap_or("--internal");

    let (mode_name, mode) = match mode_arg {
        "--internal" | "internal" => ("internal", ggnmem_db::CleanupMode::Internal),
        "--duplicates" | "duplicates" => ("duplicates", ggnmem_db::CleanupMode::Duplicates),
        "--failed" | "failed" => ("failed", ggnmem_db::CleanupMode::Failed),
        "--older-than" | "older-than" => {
            let days = args
                .get(3)
                .context("usage: ggnmem cleanup --older-than <days>")?
                .parse::<u32>()
                .context("days must be a positive integer")?;
            ("older-than", ggnmem_db::CleanupMode::OlderThan(days))
        }
        _ => bail!(
            "unknown cleanup mode: {mode_arg}\nusage:\n  ggnmem cleanup --internal\n  ggnmem cleanup --duplicates\n  ggnmem cleanup --failed\n  ggnmem cleanup --older-than DAYS"
        ),
    };

    println!("cleaning up database (mode: {mode_name})...");
    let response = request(DaemonRequest::cleanup_with_mode(mode)).await?;
    match response.kind {
        DaemonResponseKind::CleanupResult { removed, remaining } => {
            println!("removed {removed} rows. {remaining} commands remain.");
            Ok(())
        }
        DaemonResponseKind::Error { code, message } => bail!("{code}: {message}"),
        other => bail!("unexpected daemon response: {other:?}"),
    }
}

async fn optimize() -> Result<()> {
    println!("optimizing database (this may take a few seconds)...");
    let response = request(DaemonRequest::optimize_db()).await?;
    match response.kind {
        DaemonResponseKind::OptimizeResult { stats } => {
            println!("✓ database optimized in {}ms.", stats.elapsed_ms);
            println!("  before: {}", format_bytes(stats.before_size_bytes));
            println!("  after:  {}", format_bytes(stats.after_size_bytes));
            println!(
                "  vacuum: {}",
                if stats.vacuum_ran { "ran" } else { "skipped" }
            );
            Ok(())
        }
        DaemonResponseKind::Error { code, message } => bail!("{code}: {message}"),
        other => bail!("unexpected daemon response: {other:?}"),
    }
}

async fn db(args: &[String]) -> Result<()> {
    match args.get(2).map(String::as_str) {
        Some("stats") => db_stats().await,
        Some(sub) => bail!("unknown db subcommand: {sub}\n\nusage:\n  ggnmem db stats"),
        None => bail!("usage: ggnmem db stats"),
    }
}

async fn db_stats() -> Result<()> {
    let response = request(DaemonRequest::get_db_stats()).await?;

    let stats = match response.kind {
        DaemonResponseKind::DbStatsResult { stats } => stats,
        DaemonResponseKind::Error { code, message } => bail!("{code}: {message}"),
        other => bail!("unexpected daemon response: {other:?}"),
    };

    println!("ggnmem db stats");
    println!("─────────────────────────────────");
    println!(
        "  database size:      {}",
        format_bytes(stats.db_size_bytes)
    );
    println!("  rows:");
    println!("    commands:         {}", stats.command_count);
    println!("    sessions:         {}", stats.session_count);
    println!("    metadata:         {}", stats.metadata_count);
    println!("    queue:            {}", stats.queue_count);
    println!(
        "  fts estimate:       {} ({} shadow rows)",
        format_bytes(stats.fts_size_estimate()),
        stats.fts_row_count
    );
    println!(
        "  duplicate estimate: {} repeated runs",
        stats.duplicate_count_estimate
    );
    println!(
        "  pages:              {} total, {} free ({:.1}% fragmented)",
        stats.page_count,
        stats.freelist_count,
        stats.fragmentation_pct()
    );
    println!(
        "  last optimize:      {}",
        format_optional_timestamp(stats.last_optimize_at_ms)
    );

    Ok(())
}

async fn stats() -> Result<()> {
    let response = request(DaemonRequest::get_stats()).await?;
    let db_stats = request(DaemonRequest::get_db_stats()).await?;
    let config = config::load().ok();

    let (usage, uptime) = match response.kind {
        DaemonResponseKind::StatsResult { stats, uptime_ms } => (stats, uptime_ms),
        DaemonResponseKind::Error { code, message } => bail!("{code}: {message}"),
        other => bail!("unexpected daemon response: {other:?}"),
    };

    let db = match db_stats.kind {
        DaemonResponseKind::DbStatsResult { stats } => stats,
        DaemonResponseKind::Error { code, message } => bail!("{code}: {message}"),
        other => bail!("unexpected daemon response: {other:?}"),
    };

    println!("ggnmem statistics");
    println!("─────────────────────────────────");
    println!("daemon uptime:      {}s", uptime / 1000);
    println!();
    println!("usage:");
    println!("  total commands:   {}", usage.total_commands);
    println!("  unique commands:  {}", usage.unique_commands);
    println!("  searches:         {}", usage.searches_performed);
    println!("  deduplicated:     {}", usage.deduplicated_commands);
    println!("  total sessions:   {}", usage.total_sessions);
    println!();
    println!("search metrics:");
    println!("  Hybrid Searches:        {}", usage.hybrid_searches);
    println!("  Semantic Searches:      {}", usage.semantic_searches);
    println!(
        "  Average Search Latency: {}ms",
        usage.avg_search_latency_ms
    );
    println!();
    println!("database:");
    println!(
        "  size:             {:.2} MB",
        db.db_size_bytes as f64 / 1_048_576.0
    );
    println!("  fragmentation:    {:.1}%", db.fragmentation_pct());
    println!(
        "  pages (free):     {} ({} free)",
        db.page_count, db.freelist_count
    );
    println!("  fts shadow rows:  {}", db.fts_row_count);
    println!("  duplicate runs:   {}", db.duplicate_count_estimate);
    println!(
        "  last optimize:    {}",
        format_optional_timestamp(usage.last_optimize_at_ms)
    );
    println!();
    println!("retention:");
    if let Some(cfg) = config {
        println!("  retention days:   {}", cfg.retention.retention_days);
        println!("  max commands:     {}", cfg.retention.max_commands);
        println!("  auto cleanup:     {}", cfg.retention.auto_cleanup);
    } else {
        println!("  settings:         unavailable");
    }
    println!(
        "  last cleanup:     {}",
        format_optional_timestamp(usage.last_cleanup_at_ms)
    );
    println!("  last removed:     {}", usage.last_cleanup_removed);
    println!("  remaining then:   {}", usage.last_cleanup_remaining);
    println!();
    println!("most used commands:");
    for (cmd, count) in usage.most_used.iter().take(5) {
        println!("  {count:>4}x  {cmd}");
    }

    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / 1_048_576.0)
    } else {
        format!("{:.2} GB", bytes as f64 / 1_073_741_824.0)
    }
}

fn format_optional_timestamp(millis: i64) -> String {
    if millis <= 0 {
        "never".to_owned()
    } else {
        format_timestamp(millis)
    }
}

// ─── AI commands ─────────────────────────────────────────────────────────────

/// Build an `AiConfig` from the CLI config.
fn build_ai_config(cfg: &config::GgnmemConfig) -> ggnmem_ai::AiConfig {
    ggnmem_ai::AiConfig {
        enabled: cfg.ai.ai_enabled,
        embedding_provider: cfg.ai.embedding_provider.clone(),
        semantic_search: cfg.ai.semantic_search,
        model_name: cfg.ai.model_name.clone(),
        ..ggnmem_ai::AiConfig::default()
    }
}

fn ai_status() -> Result<()> {
    let cfg = config::load()?;
    let ai_cfg = build_ai_config(&cfg);
    let mgr = ggnmem_ai::ModelManager::new(ai_cfg.models_dir.clone());
    let store = ggnmem_ai::VectorStore::new(ai_cfg.vector_db_path.clone());

    // Determine provider type.
    let has_real_model = mgr.has_real_model_files(&ai_cfg.model_name);
    let provider_name = if has_real_model {
        if ai_cfg.model_name.contains("bge") {
            "BGE Small ONNX"
        } else {
            "MiniLM ONNX"
        }
    } else {
        "N-gram (fallback)"
    };
    let backend_name = if has_real_model {
        "ONNX Runtime"
    } else {
        "feature hashing"
    };

    // Get model dimensions from registry.
    let dimensions = mgr
        .get_model(&ai_cfg.model_name)
        .map(|m| m.dimensions)
        .unwrap_or(384);

    println!("ggnmem ai status");
    println!("─────────────────────────────────");
    println!(
        "  ai_enabled       ... {}",
        if ai_cfg.enabled {
            "✓ true"
        } else {
            "✗ false"
        }
    );
    println!(
        "  semantic_search  ... {}",
        if ai_cfg.semantic_search {
            "✓ true"
        } else {
            "✗ false"
        }
    );
    println!("  active model     ... {}", ai_cfg.model_name);
    println!("  provider         ... {provider_name}");
    println!("  backend          ... {backend_name}");
    println!("  dimensions       ... {dimensions}");

    // Installed models.
    let installed = mgr.list_installed();
    print!("  installed models ... ");
    if installed.is_empty() {
        println!("none");
    } else {
        let names: Vec<String> = installed
            .iter()
            .map(|m| {
                if m.name == ai_cfg.model_name {
                    format!("{} (active)", m.name)
                } else {
                    m.name.clone()
                }
            })
            .collect();
        println!("{}", names.join(", "));
    }

    // Active model details.
    let model_installed = mgr.is_installed(&ai_cfg.model_name);
    print!("  model installed  ... ");
    if model_installed {
        let size_str = mgr
            .model_size(&ai_cfg.model_name)
            .map(format_bytes)
            .unwrap_or_else(|| "—".to_owned());
        println!("✓ ({})", size_str);
    } else {
        println!("✗");
    }

    print!("  model size       ... ");
    match mgr.get_model(&ai_cfg.model_name) {
        Ok(info) => println!("~{}", format_bytes(info.size_bytes)),
        Err(_) => println!("—"),
    }

    print!("  vector db        ... ");
    // Auto-initialize when preconditions are met.
    if ai_cfg.enabled && model_installed && !store.is_initialized() {
        let _ = store.ensure_initialized();
    }
    if store.is_initialized() {
        let count = store.count().unwrap_or(0);
        println!("✓ initialized ({count} vectors)");
    } else {
        println!("✗ not initialized");
    }

    print!("  vector count     ... ");
    println!("{}", store.count().unwrap_or(0));

    // Index progress.
    let ai_cfg_path = ai_cfg.clone();
    let db_path = default_db_path();
    if db_path.exists() {
        let (provider, _) = ggnmem_ai::create_provider(&ai_cfg.models_dir, &ai_cfg.model_name);
        let progress_store = ggnmem_ai::VectorStore::new(ai_cfg_path.vector_db_path);
        let pipeline = ggnmem_ai::EmbeddingPipeline::new(provider, progress_store);
        match ggnmem_ai::indexer::get_index_progress(&db_path, &pipeline) {
            Ok(progress) => {
                println!(
                    "  index progress   ... {} / {} ({}%)",
                    progress.indexed,
                    progress.total,
                    progress.percent()
                );
            }
            Err(_) => {
                println!("  index progress   ... —");
            }
        }
    }

    Ok(())
}

fn ai_enable(args: &[String]) -> Result<()> {
    let no_install = has_flag(args, "--no-install");

    let mut cfg = config::load()?;
    cfg.ai.ai_enabled = true;
    cfg.ai.semantic_search = true;
    cfg.features.ai = true;
    config::save(&cfg)?;
    println!("  \u{2713} AI features enabled");
    println!("  ai_enabled = true");
    println!("  semantic_search = true");
    println!("  saved to {}", config::config_path()?.display());

    // Auto-initialize vector DB if model is installed.
    let ai_cfg = build_ai_config(&cfg);
    let mgr = ggnmem_ai::ModelManager::new(ai_cfg.models_dir.clone());
    if mgr.is_installed(&cfg.ai.model_name) {
        let store = ggnmem_ai::VectorStore::new(ai_cfg.vector_db_path);
        if let Err(e) = store.ensure_initialized() {
            eprintln!("  warning: could not initialize vector db: {e}");
        } else {
            println!("  \u{2713} vector db initialized");
        }
    } else if !no_install {
        // Model not installed — offer to install the recommended model.
        println!();
        println!("  AI model not installed.");
        eprint!("  Install recommended model now? [Y/n] ");

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let answer = input.trim().to_lowercase();

        if answer.is_empty() || answer == "y" || answer == "yes" {
            println!();
            do_model_install(
                &cfg.ai.model_name,
                &ai_cfg.models_dir,
                &ai_cfg.vector_db_path,
                true,
            )?;
        } else {
            println!("  skipped. Install later with: ggnmem ai install");
        }
    }
    Ok(())
}

fn ai_disable() -> Result<()> {
    let mut cfg = config::load()?;
    cfg.ai.ai_enabled = false;
    cfg.ai.semantic_search = false;
    cfg.features.ai = false;
    config::save(&cfg)?;
    println!("  \u{2713} AI features disabled");
    println!("  ai_enabled = false");
    println!("  semantic_search = false");
    println!("  saved to {}", config::config_path()?.display());
    Ok(())
}

fn ai_models() -> Result<()> {
    let cfg = config::load()?;
    let ai_cfg = build_ai_config(&cfg);
    let mgr = ggnmem_ai::ModelManager::new(ai_cfg.models_dir.clone());
    let models = mgr.list_available();

    println!("ggnmem ai models");
    println!("─────────────────────────────────");
    println!();

    for model in &models {
        let status = if model.installed {
            "✓ installed"
        } else if !model.downloadable {
            "  unavailable"
        } else {
            "  available"
        };
        let active = if model.name == ai_cfg.model_name {
            " (active)"
        } else {
            ""
        };
        let recommended = if model.name == "all-MiniLM-L6-v2" {
            " (Recommended)"
        } else {
            ""
        };
        println!("  {} {}{}{}", status, model.name, active, recommended);
        println!("    {}", model.description);
        if !model.downloadable {
            println!("    ⚠ not available for download");
        }
        println!(
            "    dimensions: {}  size: ~{}",
            model.dimensions,
            format_bytes(model.size_bytes)
        );
        if model.installed {
            if let Some(ref path) = model.install_path {
                println!("    path: {}", path.display());
            }
            if model.disk_size_bytes > 0 {
                println!("    disk: {}", format_bytes(model.disk_size_bytes));
            }
        }
        println!();
    }

    println!("  models dir: {}", ai_cfg.models_dir.display());
    println!();
    println!("  aliases: minilm, mini, bge, bge-small");
    Ok(())
}

fn ai_install(args: &[String]) -> Result<()> {
    let explicit_model = args.get(3).map(String::as_str);

    let cfg = config::load()?;
    let ai_cfg = build_ai_config(&cfg);

    // If no model name given, show interactive selection menu.
    let model_name = match explicit_model {
        Some(name) => {
            // Resolve alias (Part D).
            ggnmem_ai::resolve_alias(name)
        }
        None => {
            // Interactive model selection (Part A).
            select_model_interactive()?
        }
    };

    do_model_install(
        &model_name,
        &ai_cfg.models_dir,
        &ai_cfg.vector_db_path,
        cfg.ai.ai_enabled,
    )
}

/// Interactive model selection menu.
///
/// Displays available models with descriptions and lets the user pick one.
/// Returns the canonical model name.
fn select_model_interactive() -> Result<String> {
    println!("ggnmem ai install");
    println!("─────────────────────────────────");
    println!();
    println!("  Select a model to install:");
    println!();
    println!("  1. MiniLM (Recommended, ~80 MB)");
    println!("     all-MiniLM-L6-v2 — fast, accurate, 384 dimensions");
    println!();
    println!("  2. BGE Small (~130 MB)");
    println!("     bge-small-en-v1.5 — high quality, 384 dimensions");
    println!();
    eprint!("  Select model [1]: ");

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let choice = input.trim();

    match choice {
        "" | "1" => Ok("all-MiniLM-L6-v2".to_owned()),
        "2" => Ok("bge-small-en-v1.5".to_owned()),
        other => bail!("invalid selection: {other}. Choose 1 or 2."),
    }
}

/// Shared model installation logic used by ai_install, ai_enable, and ai_setup.
///
/// Handles: download → verify → ONNX load test → vector DB init → optional reindex.
fn do_model_install(
    model_name: &str,
    models_dir: &std::path::Path,
    vector_db_path: &std::path::Path,
    ai_enabled: bool,
) -> Result<()> {
    let canonical = ggnmem_ai::resolve_alias(model_name);
    let mgr = ggnmem_ai::ModelManager::new(models_dir.to_path_buf());

    // Detect marker-only installs that need upgrading to real ONNX files.
    let upgrading = mgr.needs_upgrade(&canonical);
    if upgrading {
        println!("  upgrading model '{canonical}' from marker to real ONNX files...");
    } else {
        println!("  installing model '{canonical}'...");
    }

    match mgr.install(&canonical, |downloaded, total| {
        if let Some(pct) = (downloaded * 100).checked_div(total) {
            eprint!(
                "\r  downloading: {} / {} ({pct}%)",
                format_bytes(downloaded),
                format_bytes(total)
            );
        } else {
            eprint!("\r  downloading: {}", format_bytes(downloaded));
        }
    }) {
        Ok(info) => {
            eprintln!(); // newline after progress
            if upgrading {
                println!(
                    "  \u{2713} model '{}' upgraded to real ONNX files",
                    info.name
                );
            } else {
                println!("  \u{2713} model '{}' installed", info.name);
            }
            if let Some(ref path) = info.install_path {
                println!("  path: {}", path.display());
            }
            println!("  size: {}", format_bytes(info.disk_size_bytes));

            // Verify integrity.
            let mgr2 = ggnmem_ai::ModelManager::new(
                info.install_path
                    .as_ref()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .to_path_buf(),
            );
            match mgr2.verify_integrity(&canonical) {
                Ok(()) => println!("  \u{2713} integrity verified"),
                Err(e) => eprintln!("  \u{26a0} integrity warning: {e}"),
            }

            // Post-install verification: load model through ort and produce
            // one real embedding to prove the full pipeline works.
            verify_model_loads(models_dir, &canonical);

            // Auto-initialize vector DB if AI is enabled.
            if ai_enabled {
                let store = ggnmem_ai::VectorStore::new(vector_db_path.to_path_buf());
                if let Err(e) = store.ensure_initialized() {
                    eprintln!("  warning: could not initialize vector db: {e}");
                } else {
                    println!("  \u{2713} vector db initialized");
                }

                // Auto-reindex if database exists.
                let db_path = default_db_path();
                if db_path.exists() {
                    println!("  reindexing embeddings...");
                    let (provider, _) = ggnmem_ai::create_provider(models_dir, &canonical);
                    let store2 = ggnmem_ai::VectorStore::new(vector_db_path.to_path_buf());
                    let pipeline = ggnmem_ai::EmbeddingPipeline::new(provider, store2);
                    match ggnmem_ai::indexer::index_all_commands(
                        &db_path,
                        &pipeline,
                        |done, total| {
                            if total > 0 {
                                eprint!("\r  indexed: {done} / {total}");
                            }
                        },
                    ) {
                        Ok(count) => {
                            eprintln!();
                            println!("  \u{2713} indexed {count} commands");
                        }
                        Err(e) => eprintln!("\n  \u{26a0} reindex warning: {e}"),
                    }
                }
            }
            Ok(())
        }
        Err(e) => bail!("{e}"),
    }
}

/// Post-install verification: load the ONNX model and produce one real embedding.
///
/// This proves the full pipeline works: tokenizer → ONNX inference → 384-dim vector.
/// Prints success/failure but does NOT fail the install if verification fails
/// (the files are already downloaded and verified by checksum).
fn verify_model_loads(models_dir: &std::path::Path, model_name: &str) {
    print!("  verifying model loads through ONNX Runtime... ");

    let (provider, provider_name) = ggnmem_ai::create_provider(models_dir, model_name);

    // Check we got the real ONNX provider, not the N-gram fallback.
    if provider_name.contains("fallback") || provider_name.contains("N-gram") {
        eprintln!("\u{26a0} fell back to {provider_name} (ONNX model may not have loaded)");
        return;
    }

    // Produce one real embedding.
    let test_phrase = "docker compose up";
    match provider.embed_query(test_phrase) {
        Ok(embedding) => {
            let dims = embedding.len();
            let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
            let sample: Vec<String> = embedding
                .iter()
                .take(4)
                .map(|v| format!("{v:.4}"))
                .collect();

            println!("\u{2713}");
            println!("  \u{2713} produced {dims}-dimensional embedding via {provider_name}");
            println!("    test phrase: \"{test_phrase}\"");
            println!("    magnitude:   {magnitude:.6} (expected \u{2248}1.0)");
            println!("    sample[0..4]: [{}]", sample.join(", "));

            if dims != 384 {
                eprintln!("  \u{26a0} unexpected dimensions: {dims} (expected 384)");
            }
            if (magnitude - 1.0).abs() > 0.01 {
                eprintln!("  \u{26a0} vector not unit-normalized: magnitude = {magnitude}");
            }
        }
        Err(e) => {
            eprintln!("\u{2717} embedding failed: {e}");
            eprintln!("  The model files are present but inference failed.");
            eprintln!("  This may indicate a corrupted download. Try:");
            eprintln!("    ggnmem ai remove {model_name} && ggnmem ai install {model_name}");
        }
    }
}

fn ai_remove(args: &[String]) -> Result<()> {
    let model_name = args.get(3).context("usage: ggnmem ai remove <model>")?;

    let cfg = config::load()?;
    let ai_cfg = build_ai_config(&cfg);
    let mgr = ggnmem_ai::ModelManager::new(ai_cfg.models_dir);

    match mgr.remove(model_name) {
        Ok(()) => {
            println!("  \u{2713} model '{model_name}' removed");
            Ok(())
        }
        Err(e) => bail!("{e}"),
    }
}

fn ai_verify_model(args: &[String]) -> Result<()> {
    let cfg = config::load()?;
    let ai_cfg = build_ai_config(&cfg);
    let model_name = args
        .get(3)
        .map(String::as_str)
        .unwrap_or(&ai_cfg.model_name);

    let mgr = ggnmem_ai::ModelManager::new(ai_cfg.models_dir.clone());

    println!("ggnmem ai verify-model");
    println!("\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}");
    println!("  model: {model_name}");

    // 1. Check model is installed.
    if !mgr.is_installed(model_name) {
        bail!("model '{model_name}' is not installed. Install with: ggnmem ai install");
    }
    println!("  \u{2713} model directory exists");

    // 2. Check for real ONNX files (not just a marker).
    if !mgr.has_real_model_files(model_name) {
        if mgr.needs_upgrade(model_name) {
            bail!(
                "model '{model_name}' has only a placeholder marker (no ONNX files).\n\
                 Upgrade with: ggnmem ai remove {model_name} && ggnmem ai install"
            );
        }
        bail!("model '{model_name}' is missing ONNX files (model.onnx / tokenizer.json)");
    }
    println!("  \u{2713} model.onnx and tokenizer.json present");

    // 3. File integrity (SHA256 sidecar check + size sanity).
    match mgr.verify_integrity(model_name) {
        Ok(()) => println!("  \u{2713} integrity verified (SHA256 + size check)"),
        Err(e) => {
            eprintln!("  \u{26a0} integrity warning: {e}");
            eprintln!("    (continuing with model load test)");
        }
    }

    // 4. Load ONNX model and produce a real embedding.
    verify_model_loads(&ai_cfg.models_dir, model_name);

    Ok(())
}

fn ai_reindex() -> Result<()> {
    let cfg = config::load()?;
    if !cfg.ai.ai_enabled {
        bail!("AI features are disabled. Enable with: ggnmem ai enable");
    }

    let ai_cfg = build_ai_config(&cfg);
    let mgr = ggnmem_ai::ModelManager::new(ai_cfg.models_dir.clone());
    if !mgr.is_installed(&cfg.ai.model_name) {
        bail!(
            "model '{}' is not installed. Install with: ggnmem ai install",
            cfg.ai.model_name
        );
    }

    let db_path = default_db_path();
    if !db_path.exists() {
        bail!(
            "database not found at {}. Start the daemon first.",
            db_path.display()
        );
    }

    let (provider, provider_name) =
        ggnmem_ai::create_provider(&ai_cfg.models_dir, &ai_cfg.model_name);
    let store = ggnmem_ai::VectorStore::new(ai_cfg.vector_db_path);
    let pipeline = ggnmem_ai::EmbeddingPipeline::new(provider, store);

    println!("ggnmem ai reindex");
    println!("\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}");
    println!("  provider: {provider_name}");
    println!("  deleting existing embeddings...");

    match ggnmem_ai::indexer::reindex_all_commands(&db_path, &pipeline, |done, total| {
        if total > 0 {
            eprint!("\r  indexed: {done} / {total}");
        }
    }) {
        Ok(count) => {
            eprintln!();
            println!("  complete.");
            println!("  \u{2713} indexed {count} commands");
            println!("  vector count: {}", pipeline.vector_count().unwrap_or(0));
            Ok(())
        }
        Err(e) => bail!("reindex failed: {e}"),
    }
}

// ─── AI Setup Wizard (Phase 16C) ─────────────────────────────────────────────

/// `ggnmem ai setup` — guided AI setup wizard.
///
/// Walks the user through a 5-step process:
///   1. Choose model
///   2. Download
///   3. Verify
///   4. Reindex embeddings
///   5. Test semantic search
fn ai_setup() -> Result<()> {
    println!("ggnmem ai setup");
    println!("─────────────────────────────────");
    println!("  AI Setup Wizard");
    println!();

    // ── Step 1: Choose model ──
    println!("  Step 1/5: Choose model");
    println!();
    println!("  1. MiniLM (Recommended, ~80 MB)");
    println!("     all-MiniLM-L6-v2 — fast, accurate, 384 dimensions");
    println!();
    println!("  2. BGE Small (~130 MB)");
    println!("     bge-small-en-v1.5 — high quality, 384 dimensions");
    println!();
    eprint!("  Select model [1]: ");

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let choice = input.trim();

    let model_name = match choice {
        "" | "1" => "all-MiniLM-L6-v2".to_owned(),
        "2" => "bge-small-en-v1.5".to_owned(),
        other => bail!("invalid selection: {other}. Choose 1 or 2."),
    };

    println!();
    println!("  ✓ Selected: {model_name}");

    // Enable AI in config.
    let mut cfg = config::load()?;
    cfg.ai.ai_enabled = true;
    cfg.ai.semantic_search = true;
    cfg.features.ai = true;
    cfg.ai.model_name = model_name.to_owned();
    config::save(&cfg)?;
    println!("  ✓ AI features enabled in config");

    let ai_cfg = build_ai_config(&cfg);
    let mgr = ggnmem_ai::ModelManager::new(ai_cfg.models_dir.clone());

    // ── Step 2: Download ──
    println!();
    println!("  Step 2/5: Download");

    if mgr.is_installed(&model_name) && mgr.has_real_model_files(&model_name) {
        println!("  ✓ Model already installed");
    } else {
        do_model_install(
            &model_name,
            &ai_cfg.models_dir,
            &ai_cfg.vector_db_path,
            false, // We'll handle reindex ourselves in step 4
        )?;
    }

    // ── Step 3: Verify ──
    println!();
    println!("  Step 3/5: Verify");

    match mgr.verify_integrity(&model_name) {
        Ok(()) => println!("  ✓ SHA256 integrity verified"),
        Err(e) => eprintln!("  ⚠ integrity warning: {e}"),
    }

    verify_model_loads(&ai_cfg.models_dir, &model_name);

    // ── Step 4: Reindex ──
    println!();
    println!("  Step 4/5: Reindex embeddings");

    let db_path = default_db_path();
    if db_path.exists() {
        let (provider, provider_name) = ggnmem_ai::create_provider(&ai_cfg.models_dir, &model_name);
        let store = ggnmem_ai::VectorStore::new(ai_cfg.vector_db_path.clone());
        let _ = store.ensure_initialized();
        let pipeline = ggnmem_ai::EmbeddingPipeline::new(provider, store);

        println!("  provider: {provider_name}");

        match ggnmem_ai::indexer::index_all_commands(&db_path, &pipeline, |done, total| {
            if total > 0 {
                eprint!("\r  indexed: {done} / {total}");
            }
        }) {
            Ok(count) => {
                eprintln!();
                if count > 0 {
                    println!("  ✓ indexed {count} commands");
                } else {
                    println!("  ✓ all commands already indexed");
                }
            }
            Err(e) => eprintln!("  ⚠ reindex warning: {e}"),
        }
    } else {
        println!("  — database not found (start daemon first to capture commands)");
        println!("    embeddings will be built as commands are captured");
    }

    // ── Step 5: Test semantic search ──
    println!();
    println!("  Step 5/5: Test semantic search");

    if db_path.exists() {
        let (provider, _) = ggnmem_ai::create_provider(&ai_cfg.models_dir, &model_name);
        let store = ggnmem_ai::VectorStore::new(ai_cfg.vector_db_path.clone());
        let pipeline = ggnmem_ai::EmbeddingPipeline::new(provider, store);

        let test_query = "list files";
        match pipeline.search_embedding(test_query, 3) {
            Ok(matches) => {
                if matches.is_empty() {
                    println!("  ✓ semantic search operational (no results yet — capture some commands first)");
                } else {
                    println!(
                        "  ✓ semantic search operational — {} result(s) for '{test_query}':",
                        matches.len()
                    );

                    // Show top results with command text from DB.
                    let database =
                        ggnmem_db::Database::open(&ggnmem_db::DatabaseConfig::new(db_path))?;
                    for (i, m) in matches.iter().take(3).enumerate() {
                        if let Ok(Some(cmd)) = database.get_command_by_id(&m.id) {
                            let sim = (1.0 - m.distance as f64) * 100.0;
                            println!("    {}. [{:.0}%] {}", i + 1, sim, cmd.command);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("  ⚠ semantic search test failed: {e}");
                eprintln!("    try: ggnmem ai reindex");
            }
        }
    } else {
        println!("  — skipped (no database yet)");
    }

    println!();
    println!("─────────────────────────────────");
    println!("  ✓ AI setup complete!");
    println!();
    println!("  Next steps:");
    println!("    ggnmem start        Start the daemon");
    println!("    ggnmem search       Search with hybrid FTS + semantic");
    println!("    ggnmem ai status    Check AI status");

    Ok(())
}

// ─── AI Doctor (Phase 16G) ───────────────────────────────────────────────────

/// `ggnmem ai doctor` — run AI diagnostic checks.
///
/// Checks:
///   1. Model files exist on disk
///   2. SHA256 checksums valid
///   3. ONNX session loads
///   4. Embedding generation works
///   5. Vector DB is healthy
fn ai_doctor() -> Result<()> {
    println!("ggnmem ai doctor");
    println!("─────────────────────────────────");
    println!();

    let cfg = config::load()?;
    let ai_cfg = build_ai_config(&cfg);
    let mgr = ggnmem_ai::ModelManager::new(ai_cfg.models_dir.clone());

    let model_name = &cfg.ai.model_name;
    let mut all_ok = true;

    // ── Check 1: Model files exist ──
    print!("  1. model exists       ... ");
    if mgr.is_installed(model_name) {
        let size = mgr
            .model_size(model_name)
            .map(format_bytes)
            .unwrap_or_else(|| "—".to_owned());
        println!("✓ {model_name} ({size})");
    } else {
        println!("✗ model '{model_name}' not installed");
        println!("    install with: ggnmem ai install");
        all_ok = false;
    }

    let has_real = mgr.has_real_model_files(model_name);
    print!("     ONNX files         ... ");
    if has_real {
        println!("✓ model.onnx + tokenizer.json");
    } else if mgr.needs_upgrade(model_name) {
        println!("⚠ marker only (needs upgrade: ggnmem ai install)");
        all_ok = false;
    } else if !mgr.is_installed(model_name) {
        println!("— not installed");
    } else {
        println!("✗ missing");
        all_ok = false;
    }

    // ── Check 2: Checksum valid ──
    print!("  2. checksum valid     ... ");
    if mgr.is_installed(model_name) {
        match mgr.verify_integrity(model_name) {
            Ok(()) => println!("✓ SHA256 verified"),
            Err(e) => {
                println!("✗ {e}");
                all_ok = false;
            }
        }
    } else {
        println!("— skipped (not installed)");
    }

    // ── Check 3: ONNX loads ──
    print!("  3. ONNX loads         ... ");
    if has_real {
        let (provider, provider_name) = ggnmem_ai::create_provider(&ai_cfg.models_dir, model_name);

        if provider_name.contains("fallback") || provider_name.contains("N-gram") {
            println!("⚠ fell back to {provider_name}");
            all_ok = false;
        } else {
            println!("✓ {provider_name}");

            // ── Check 4: Embedding generation ──
            print!("  4. embedding works    ... ");
            let test_phrase = "docker compose up";
            match provider.embed_query(test_phrase) {
                Ok(embedding) if !embedding.is_empty() => {
                    let dims = embedding.len();
                    let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();

                    if dims == 384 && (magnitude - 1.0).abs() < 0.05 {
                        println!("✓ {dims}d, magnitude={magnitude:.4}");
                    } else {
                        println!("⚠ {dims}d, magnitude={magnitude:.4} (expected 384d, ~1.0)");
                        all_ok = false;
                    }
                }
                Ok(_) => {
                    println!("✗ produced empty embedding");
                    all_ok = false;
                }
                Err(e) => {
                    println!("✗ {e}");
                    all_ok = false;
                }
            }
        }
    } else {
        println!("— skipped (no ONNX files)");
        println!("  4. embedding works    ... — skipped");
    }

    // ── Check 5: Vector DB healthy ──
    print!("  5. vector DB healthy  ... ");
    let store = ggnmem_ai::VectorStore::new(ai_cfg.vector_db_path.clone());
    if store.is_initialized() {
        match store.count() {
            Ok(count) => println!("✓ initialized ({count} vectors)"),
            Err(e) => {
                println!("✗ count failed: {e}");
                all_ok = false;
            }
        }
    } else {
        // Try to initialize.
        match store.ensure_initialized() {
            Ok(()) => {
                let count = store.count().unwrap_or(0);
                println!("✓ initialized ({count} vectors)");
            }
            Err(e) => {
                println!("✗ initialization failed: {e}");
                all_ok = false;
            }
        }
    }

    println!();
    println!("─────────────────────────────────");
    if all_ok {
        println!("  ✓ all checks passed");
    } else {
        println!("  ✗ some checks failed — review above");
    }

    Ok(())
}

// ─── Semantic search (CLI-direct, no daemon needed) ──────────────────────────

async fn semantic(args: &[String]) -> Result<()> {
    let limit = parse_named_arg(args, "--limit")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(10);
    let json_output = has_flag(args, "--json");

    // Build query from positional args.
    let valued_flags = ["--limit"];
    let boolean_flags = ["--json"];

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
        bail!("usage: ggnmem semantic <query> [--limit N] [--json]");
    }

    let cfg = config::load()?;
    if !cfg.ai.ai_enabled {
        bail!("AI features are disabled. Enable with: ggnmem ai enable");
    }

    let ai_cfg = build_ai_config(&cfg);
    let db_path = default_db_path();
    if !db_path.exists() {
        bail!(
            "database not found at {}. Start the daemon first.",
            db_path.display()
        );
    }

    let (provider, provider_name) =
        ggnmem_ai::create_provider(&ai_cfg.models_dir, &ai_cfg.model_name);
    let store = ggnmem_ai::VectorStore::new(ai_cfg.vector_db_path);
    let pipeline = ggnmem_ai::EmbeddingPipeline::new(provider, store);

    // Embed query and search vector store.
    let start = std::time::Instant::now();
    let matches = pipeline
        .search_embedding(&query, limit as usize + 10)
        .context("semantic search failed")?;
    let elapsed = start.elapsed();

    if matches.is_empty() {
        println!("no semantic results for: {query}");
        println!("(run `ggnmem ai reindex` to build the embedding index)");
        return Ok(());
    }

    // Cross-reference with commands DB for metadata.
    let database = ggnmem_db::Database::open(&ggnmem_db::DatabaseConfig::new(db_path))?;
    let mut results: Vec<SemanticDisplayResult> = Vec::new();

    for m in &matches {
        if let Ok(Some(cmd)) = database.get_command_by_id(&m.id) {
            let similarity = (1.0 - m.distance as f64).clamp(0.0, 1.0);
            results.push(SemanticDisplayResult {
                command: cmd.command,
                cwd: cmd.cwd,
                exit_code: cmd.exit_code,
                duration_ms: cmd.duration_ms,
                completed_at_ms: cmd.completed_at_ms,
                similarity,
            });
        }
    }

    results.truncate(limit as usize);

    if json_output {
        let json =
            serde_json::to_string_pretty(&results).context("serialize semantic results to JSON")?;
        println!("{json}");
        return Ok(());
    }

    println!(
        "found {} semantic result{} for: {query}  ({provider_name}, {elapsed:.1?})",
        results.len(),
        if results.len() == 1 { "" } else { "s" }
    );
    println!();

    for result in &results {
        let exit_str = result
            .exit_code
            .map(|c| {
                if c == 0 {
                    "  \u{2713} ".to_owned()
                } else {
                    format!("\u{2717}{c:>2} ")
                }
            })
            .unwrap_or_else(|| " ?  ".to_owned());
        let ts = format_timestamp(result.completed_at_ms);
        let dur = format_duration(result.duration_ms);
        let sim_pct = (result.similarity * 100.0) as u32;

        println!(
            "  {exit_str} {ts}  {dur:>7}  [sim {sim_pct:>3}%]  {cwd}",
            cwd = result.cwd
        );
        println!("       $ {cmd}", cmd = result.command);
        println!();
    }

    Ok(())
}

/// Lightweight struct for displaying semantic search results.
#[derive(Debug, Clone, serde::Serialize)]
struct SemanticDisplayResult {
    command: String,
    cwd: String,
    exit_code: Option<i32>,
    duration_ms: Option<i64>,
    completed_at_ms: i64,
    similarity: f64,
}

// ─── Knowledge Base commands (Phase 18) ──────────────────────────────────────

/// `ggnmem ask "<query>"` — Ask the knowledge base for a command suggestion.
fn cmd_ask(args: &[String]) -> Result<()> {
    let query = args
        .iter()
        .skip(2)
        .cloned()
        .collect::<Vec<String>>()
        .join(" ");

    if query.is_empty() {
        println!("usage: ggnmem ask \"<natural language query>\"\n\nexamples:\n  ggnmem ask \"show running containers\"\n  ggnmem ask \"check git changes\"\n  ggnmem ask \"build rust project\"");
        return Ok(());
    }

    let kb = ggnmem_knowledge::KnowledgeBase::new();
    let results = kb.ask(&query, 5);

    if results.is_empty() {
        println!("no suggestions found for: {query}");
        println!();
        println!("  available topics: docker, git, linux, cargo, go, kubernetes");
        println!("  you can add custom knowledge packs under: ~/.config/ggnmem/knowledge/");
        return Ok(());
    }

    println!("ggnmem ask");
    println!("─────────────────────────────────");
    println!("  query: {query}");
    println!();

    for (i, result) in results.iter().enumerate() {
        let conf = ggnmem_knowledge::format_confidence(result.confidence);
        if i == 0 {
            // Primary suggestion — highlighted.
            println!("  ┌─────────────────────────────────────────────┐");
            println!("  │  Suggested:   $ {}", result.command);
            println!("  │  Description: {}", result.description);
            println!("  │  Category:    {} / {}", result.topic, result.category);
            println!("  │  Confidence:  {conf}");
            println!("  │  Source:      {}", result.source);
            println!("  └─────────────────────────────────────────────┘");
        } else {
            println!();
            println!("  {}.  $ {}", i + 1, result.command);
            println!("      {}", result.description);
            println!("      Confidence: {conf}");
        }
    }

    println!();
    Ok(())
}

/// `ggnmem explain "<command>"` — Explain what a command does.
fn cmd_explain(args: &[String]) -> Result<()> {
    let command = args
        .iter()
        .skip(2)
        .cloned()
        .collect::<Vec<String>>()
        .join(" ");

    if command.is_empty() {
        println!("usage: ggnmem explain \"<command>\"\n\nexamples:\n  ggnmem explain \"docker ps\"\n  ggnmem explain \"git status\"\n  ggnmem explain \"cargo build\"");
        return Ok(());
    }

    let kb = ggnmem_knowledge::KnowledgeBase::new();
    let result = kb.explain(&command);

    match result {
        Some(info) => {
            println!("ggnmem explain");
            println!("─────────────────────────────────");
            println!("  command:  {}", info.command);
            println!("  purpose:  {}", info.purpose);
            println!("  category: {} / {}", info.topic, info.category);

            if !info.flags.is_empty() {
                println!();
                println!("  common flags:");
                for flag in &info.flags {
                    println!("    {:<20} {}", flag.flag, flag.description);
                }
            }

            if !info.examples.is_empty() {
                println!();
                println!("  examples:");
                for example in &info.examples {
                    println!("    $ {example}");
                }
            }

            println!();
        }
        None => {
            println!("no explanation found for: {command}");
            println!();
            println!("  try searching with: ggnmem ask \"{command}\"");

            // Offer fuzzy matches.
            let results = kb.ask(&command, 3);
            if !results.is_empty() {
                println!();
                println!("  did you mean:");
                for r in &results {
                    println!("    ggnmem explain \"{}\"", r.command);
                }
            }
        }
    }

    Ok(())
}

/// `ggnmem learn <topic>` — Learn commands for a topic.
fn cmd_learn(args: &[String]) -> Result<()> {
    let topic = args.get(2).map(String::as_str);

    let kb = ggnmem_knowledge::KnowledgeBase::new();

    match topic {
        Some(t) => {
            let result = kb.learn(t);
            match result {
                Some(info) => {
                    println!("ggnmem learn: {}", info.topic);
                    println!("─────────────────────────────────");
                    println!("  {}", info.description);
                    println!();

                    for cat in &info.categories {
                        println!("  ── {} ──", cat.name);
                        println!();
                        for cmd in &cat.commands {
                            let level = match cmd.difficulty {
                                1 => "●  ",
                                2 => "●● ",
                                3 => "●●●",
                                _ => "●  ",
                            };
                            println!("    {level}  {:<30} {}", cmd.command, cmd.description);
                        }
                        println!();
                    }

                    println!("  legend: ● beginner  ●● intermediate  ●●● advanced");
                    println!();
                }
                None => {
                    println!("unknown topic: {t}");
                    println!();
                    print_available_topics(&kb);
                }
            }
        }
        None => {
            println!("ggnmem learn");
            println!("─────────────────────────────────");
            println!();
            print_available_topics(&kb);
            println!();
            println!("  usage: ggnmem learn <topic>");
        }
    }

    Ok(())
}

/// Print available knowledge topics.
fn print_available_topics(kb: &ggnmem_knowledge::KnowledgeBase) {
    println!("  available topics:");
    println!();
    for (name, desc) in kb.topics() {
        println!("    {name:<15} {desc}");
    }
    println!();
    println!("  total entries: {}", kb.entry_count());
    println!("  custom packs:  ~/.config/ggnmem/knowledge/*.json or *.toml");
}

/// `ggnmem knowledge <list|validate>` — Manage knowledge packs.
fn cmd_knowledge(args: &[String]) -> Result<()> {
    match args.get(2).map(String::as_str) {
        Some("list") => {
            let kb = ggnmem_knowledge::KnowledgeBase::new();
            println!("ggnmem knowledge list");
            println!("─────────────────────────────────");
            println!();

            // Show all pack sources with details.
            let sources = kb.pack_sources();
            let builtin: Vec<_> = sources.iter().filter(|s| s.source == "builtin").collect();
            let custom: Vec<_> = sources.iter().filter(|s| s.source != "builtin").collect();

            println!("  built-in packs ({}):", builtin.len());
            for ps in &builtin {
                println!("    {:<15} {} entries", ps.name, ps.entry_count);
            }
            println!();

            if custom.is_empty() {
                println!("  custom packs:  (none)");
            } else {
                println!("  custom packs ({}):", custom.len());
                for ps in &custom {
                    println!(
                        "    {:<15} {} entries  ← {}",
                        ps.name, ps.entry_count, ps.source
                    );
                }
            }
            println!();

            // Show user dir.
            match ggnmem_knowledge::KnowledgeBase::user_dir() {
                Some(dir) => {
                    println!("  user dir: {}", dir.display());
                    if !dir.exists() {
                        println!("  (directory does not exist — create it to add custom packs)");
                    }
                }
                None => println!("  user dir: (could not determine)"),
            }
            println!();
            println!("  total entries: {}", kb.entry_count());

            // Show errors if any.
            let errors = kb.load_errors();
            if !errors.is_empty() {
                println!();
                println!("  ⚠ {} error(s) loading packs:", errors.len());
                for err in errors {
                    println!("    • {err}");
                }
            }

            Ok(())
        }
        Some("validate") => {
            let kb = ggnmem_knowledge::KnowledgeBase::new();
            println!("ggnmem knowledge validate");
            println!("─────────────────────────────────");

            let sources = kb.pack_sources();
            let errors = kb.load_errors();

            println!("  {} packs loaded successfully.", sources.len());
            println!("  {} entries indexed.", kb.entry_count());

            // Per-pack validation.
            for ps in sources {
                println!(
                    "    ✓ {:<15} {} entries  ({})",
                    ps.name, ps.entry_count, ps.source
                );
            }

            if errors.is_empty() {
                println!();
                println!("  status: OK");
            } else {
                println!();
                println!("  ⚠ {} error(s):", errors.len());
                for err in errors {
                    println!("    ✗ {err}");
                }
                println!();
                println!("  status: PARTIAL (some packs failed to load)");
                println!();
                println!("  Custom pack format (JSON):");
                println!("    Option 1 — full format:");
                println!(
                    "      {{\"topic\": \"name\", \"description\": \"...\", \"entries\": [...]}}"
                );
                println!("    Option 2 — simple array:");
                println!("      [{{\"command\": \"cmd\", \"description\": \"...\"}}, ...]");
            }

            Ok(())
        }
        Some(sub) => {
            println!("unknown knowledge subcommand: {sub}\n\nusage:\n  ggnmem knowledge list\n  ggnmem knowledge validate");
            Ok(())
        }
        None => {
            println!("usage:\n  ggnmem knowledge list\n  ggnmem knowledge validate");
            Ok(())
        }
    }
}

// ─── AI use / benchmark (Phase 18) ──────────────────────────────────────────

/// `ggnmem ai use <model>` — Switch the active embedding model.
fn ai_use(args: &[String]) -> Result<()> {
    let model_name = match args.get(3) {
        Some(name) => name,
        None => {
            println!("usage: ggnmem ai use <model>\n\nexample:\n  ggnmem ai use all-MiniLM-L6-v2");
            return Ok(());
        }
    };

    let canonical = ggnmem_ai::resolve_alias(model_name);

    // Validate model exists in registry.
    let cfg = config::load()?;
    let ai_cfg = build_ai_config(&cfg);
    let mgr = ggnmem_ai::ModelManager::new(ai_cfg.models_dir.clone());

    let model_info = mgr.get_model(&canonical);
    if model_info.is_err() {
        bail!("unknown model: {model_name}\n\navailable models:\n  all-MiniLM-L6-v2\n  bge-small-en-v1.5");
    }

    if !mgr.is_installed(&canonical) {
        bail!("model '{canonical}' is not installed.\nInstall with: ggnmem ai install {canonical}");
    }

    // Check if already active.
    if cfg.ai.model_name == canonical {
        println!("  model '{canonical}' is already active.");
        return Ok(());
    }

    let old_model = cfg.ai.model_name.clone();

    // Update config.
    let mut new_cfg = cfg;
    new_cfg.ai.model_name = canonical.clone();
    config::save(&new_cfg)?;

    println!("ggnmem ai use");
    println!("─────────────────────────────────");
    println!("  switched: {old_model} → {canonical}");
    println!("  saved to {}", config::config_path()?.display());

    // Warn about vector incompatibility.
    let store = ggnmem_ai::VectorStore::new(ai_cfg.vector_db_path.clone());
    if store.is_initialized() {
        let count = store.count().unwrap_or(0);
        if count > 0 {
            println!();
            println!("  ⚠ Warning: {count} existing embeddings were built with '{old_model}'.");
            println!("    Embeddings from different models are NOT compatible.");
            println!("    Run `ggnmem ai reindex` to rebuild embeddings with '{canonical}'.");
        }
    }

    Ok(())
}

/// `ggnmem ai benchmark` — Compare installed models.
fn ai_benchmark() -> Result<()> {
    let cfg = config::load()?;
    let ai_cfg = build_ai_config(&cfg);
    let mgr = ggnmem_ai::ModelManager::new(ai_cfg.models_dir.clone());
    let installed = mgr.list_installed();

    if installed.is_empty() {
        println!("ggnmem ai benchmark");
        println!("─────────────────────────────────");
        println!("  no models installed. Install with: ggnmem ai install");
        return Ok(());
    }

    println!("ggnmem ai benchmark");
    println!("─────────────────────────────────");
    println!();

    let test_phrases = [
        "docker compose up",
        "git push origin main",
        "cargo test --workspace",
        "kubectl get pods",
        "find . -name '*.rs'",
        "check running containers",
        "build rust project",
        "show git changes",
        "list kubernetes services",
        "compress directory",
    ];

    println!(
        "  {:<25} {:>10} {:>12} {:>10} {:>8}",
        "Model", "Load (ms)", "Embed (ms)", "Memory", "Dims"
    );
    println!("  {}", "─".repeat(70));

    for model in &installed {
        // Measure model load time.
        let load_start = std::time::Instant::now();
        let (provider, provider_name) = ggnmem_ai::create_provider(&ai_cfg.models_dir, &model.name);
        let load_ms = load_start.elapsed().as_millis();

        // Skip N-gram fallback models (no real model files).
        if provider_name.contains("fallback") || provider_name.contains("N-gram") {
            println!(
                "  {:<25} {:>10} {:>12} {:>10} {:>8}",
                model.name, "—", "—", "—", "—"
            );
            println!("    (N-gram fallback — no ONNX model loaded)");
            continue;
        }

        // Measure embedding latency (average of test phrases).
        let mut total_embed_us: u128 = 0;
        let mut dims = 0;
        for phrase in &test_phrases {
            let embed_start = std::time::Instant::now();
            if let Ok(embedding) = provider.embed_query(phrase) {
                total_embed_us += embed_start.elapsed().as_micros();
                dims = embedding.len();
            }
        }
        let avg_embed_ms = total_embed_us as f64 / test_phrases.len() as f64 / 1000.0;

        // Memory: use on-disk model size as a proxy.
        let mem_str = format_bytes(model.disk_size_bytes);

        let active = if model.name == cfg.ai.model_name {
            " (active)"
        } else {
            ""
        };

        println!(
            "  {:<25} {:>10} {:>10.2}ms {:>10} {:>8}",
            format!("{}{active}", model.name),
            load_ms,
            avg_embed_ms,
            mem_str,
            dims
        );
    }

    println!();
    println!("  test phrases: {}", test_phrases.len());
    println!("  benchmark completed.");

    Ok(())
}

/// Get the default database path (`~/.local/share/ggnmem/ggnmem.db`).
fn default_db_path() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("~"));
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local").join("share"));
    data_home.join("ggnmem").join("ggnmem.db")
}

// ─── IPC helper ──────────────────────────────────────────────────────────────

async fn request(request: DaemonRequest) -> Result<DaemonResponse> {
    let config = DaemonConfig::load().context("load daemon client configuration")?;
    if request_needs_protocol_preflight(&request) {
        ensure_daemon_protocol(&config).await?;
    }

    let mut client = IpcClient::connect(&config.endpoint)
        .await
        .context("connect to ggnmem daemon")?;
    let response: DaemonResponse = client
        .request(&request)
        .await
        .context("daemon request failed")?;
    ensure_response_protocol(response)
}

fn request_needs_protocol_preflight(request: &DaemonRequest) -> bool {
    matches!(
        request,
        DaemonRequest::SearchCommands { .. }
            | DaemonRequest::GetDbStats { .. }
            | DaemonRequest::GetStats { .. }
    )
}

async fn ensure_daemon_protocol(config: &DaemonConfig) -> Result<()> {
    let mut client = IpcClient::connect(&config.endpoint)
        .await
        .context("connect to ggnmem daemon")?;
    let response: DaemonResponse = client
        .request(&DaemonRequest::ping())
        .await
        .context("daemon protocol check failed")?;
    let response = ensure_response_protocol(response)?;
    match response.kind {
        DaemonResponseKind::Pong => Ok(()),
        DaemonResponseKind::Error { code, message } => bail!("{code}: {message}"),
        other => bail!("unexpected daemon protocol check response: {other:?}"),
    }
}

fn ensure_response_protocol(response: DaemonResponse) -> Result<DaemonResponse> {
    if response.version != PROTOCOL_VERSION {
        bail!(
            "daemon protocol mismatch: CLI uses IPC protocol v{}, daemon uses v{}. Restart the daemon and CLI from the same build.",
            PROTOCOL_VERSION,
            response.version
        );
    }
    Ok(response)
}
