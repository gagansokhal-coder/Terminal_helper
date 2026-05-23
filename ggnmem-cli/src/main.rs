mod hooks;
mod tui;

use anyhow::{bail, Context, Result};
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
        Some(command) => bail!("unknown command: {command}"),
        None => {
            print_usage();
            Ok(())
        }
    }
}

fn print_usage() {
    println!("ggnmem — semantic terminal memory engine");
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
    println!("  doctor           Check daemon, IPC, and database health");
    println!("  ping             Ping the daemon");
    println!("  status           Show daemon status");
    println!("  ingest           (internal) Ingest a command from shell hook");
    println!();
    println!("search options:");
    println!("  --limit N        Maximum results (default: 20)");
    println!("  --cwd            Boost results from current directory");
    println!("  --recent         Sort by recency only");
    println!("  --json           Output as JSON");
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

    // Check IPC connectivity.
    print!("ipc connection  ... ");
    let health_result = request(DaemonRequest::health()).await;
    match &health_result {
        Ok(response) => match &response.kind {
            DaemonResponseKind::Health(status) => {
                println!("ok");
                println!();
                println!("daemon state    ... {:?}", status.state);
                println!("uptime          ... {}ms", status.uptime_ms);
                println!(
                    "queue           ... {}/{}",
                    status.queue_depth, status.queue_capacity
                );
                println!(
                    "database        ... {}",
                    if status.db_connected {
                        "connected"
                    } else {
                        "disconnected"
                    }
                );
                println!("platform        ... {}", status.platform);
            }
            DaemonResponseKind::Error { code, message } => {
                println!("error: {code}: {message}");
            }
            other => {
                println!("unexpected: {other:?}");
            }
        },
        Err(error) => {
            println!("FAILED");
            println!("  daemon is not reachable: {error}");
            println!();
            println!("  make sure the daemon is running:");
            println!("    ggnmem-daemon");
            return Ok(());
        }
    }

    // Check command count.
    println!();
    print!("command count   ... ");
    match request(DaemonRequest::count_commands()).await {
        Ok(response) => match response.kind {
            DaemonResponseKind::CommandCount { count } => {
                println!("{count} commands indexed");
            }
            DaemonResponseKind::Error { code, message } => {
                println!("error: {code}: {message}");
            }
            _ => println!("unexpected response"),
        },
        Err(error) => println!("FAILED: {error}"),
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
