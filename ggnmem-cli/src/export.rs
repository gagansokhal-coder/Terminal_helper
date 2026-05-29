//! Export command history in JSON or CSV format.
//!
//! `ggnmem export`               — export as JSON (default).
//! `ggnmem export --format json`  — export as JSON.
//! `ggnmem export --format csv`   — export as CSV.

use anyhow::{bail, Context, Result};

use ggnmem_daemon::{
    protocol::{DaemonRequest, DaemonResponse, DaemonResponseKind},
    DaemonConfig, IpcClient,
};

// ─── Export ──────────────────────────────────────────────────────────────────

pub async fn cmd_export(args: &[String]) -> Result<()> {
    let format = parse_format(args);

    // Request a large batch of recent commands from the daemon.
    let limit = parse_limit(args);
    let config = DaemonConfig::load().context("load daemon configuration")?;
    let mut client = IpcClient::connect(&config.endpoint)
        .await
        .context("connect to ggnmem daemon — is it running?")?;

    let response: DaemonResponse = client
        .request(&DaemonRequest::query_recent(limit))
        .await
        .context("query recent commands")?;

    match response.kind {
        DaemonResponseKind::RecentCommands { commands } => {
            if commands.is_empty() {
                eprintln!("no commands to export");
                return Ok(());
            }

            match format {
                ExportFormat::Json => export_json(&commands)?,
                ExportFormat::Csv => export_csv(&commands)?,
            }

            eprintln!("exported {} commands", commands.len());
            Ok(())
        }
        DaemonResponseKind::Error { code, message } => bail!("{code}: {message}"),
        other => bail!("unexpected daemon response: {other:?}"),
    }
}

// ─── Formats ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
enum ExportFormat {
    Json,
    Csv,
}

fn parse_format(args: &[String]) -> ExportFormat {
    for (i, arg) in args.iter().enumerate() {
        if arg == "--format" {
            if let Some(fmt) = args.get(i + 1) {
                return match fmt.to_lowercase().as_str() {
                    "csv" => ExportFormat::Csv,
                    _ => ExportFormat::Json,
                };
            }
        }
    }
    ExportFormat::Json
}

fn parse_limit(args: &[String]) -> u32 {
    for (i, arg) in args.iter().enumerate() {
        if arg == "--limit" {
            if let Some(val) = args.get(i + 1) {
                if let Ok(n) = val.parse::<u32>() {
                    return n;
                }
            }
        }
    }
    // Default: export up to 100,000 commands.
    100_000
}

// ─── JSON export ─────────────────────────────────────────────────────────────

fn export_json(commands: &[ggnmem_daemon::protocol::CommandSummary]) -> Result<()> {
    let json = serde_json::to_string_pretty(commands).context("serialize to JSON")?;
    println!("{json}");
    Ok(())
}

// ─── CSV export ──────────────────────────────────────────────────────────────

fn export_csv(commands: &[ggnmem_daemon::protocol::CommandSummary]) -> Result<()> {
    // Header.
    println!("command,cwd,exit_code,duration_ms,completed_at_ms,session_id");

    for cmd in commands {
        let command = csv_escape(&cmd.command);
        let cwd = csv_escape(&cmd.cwd);
        let exit_code = cmd.exit_code.map(|c| c.to_string()).unwrap_or_default();
        let duration_ms = cmd.duration_ms.map(|d| d.to_string()).unwrap_or_default();
        let session_id = csv_escape(&cmd.session_id);

        println!(
            "{command},{cwd},{exit_code},{duration_ms},{},{session_id}",
            cmd.completed_at_ms,
        );
    }

    Ok(())
}

/// Escape a string for CSV: wrap in quotes if it contains commas, quotes, or newlines.
fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_owned()
    }
}
