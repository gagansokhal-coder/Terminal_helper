//! Shell history import for Bash, Zsh, and Fish.
//!
//! Usage:
//!   ggnmem import auto            Auto-detect shell and import
//!   ggnmem import bash            Import from ~/.bash_history
//!   ggnmem import zsh             Import from ~/.zsh_history
//!   ggnmem import fish            Import from ~/.local/share/fish/fish_history
//!   ggnmem import bash --dry-run  Show counts without modifying DB
//!   ggnmem import bash --preview  Show sample commands before importing

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use ggnmem_db::{CommandId, Database, DatabaseConfig, NewCommand, SessionId};

// ─── Public entry point ──────────────────────────────────────────────────────

pub fn cmd_import(args: &[String]) -> Result<()> {
    // Handle --help anywhere in the argument list.
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_import_help();
        return Ok(());
    }

    let shell = match args.get(2).map(String::as_str) {
        Some("auto") => detect_shell()?,
        Some("bash") => Shell::Bash,
        Some("zsh") => Shell::Zsh,
        Some("fish") => Shell::Fish,
        Some(other) => bail!(
            "unknown shell: {other}\n\nusage:\n  ggnmem import auto\n  ggnmem import bash\n  ggnmem import zsh\n  ggnmem import fish"
        ),
        None => {
            print_import_help();
            return Ok(());
        }
    };

    let dry_run = has_flag(args, "--dry-run");
    let preview = has_flag(args, "--preview");
    let custom_file = parse_named_arg(args, "--file");

    let history_path = match custom_file {
        Some(p) => PathBuf::from(p),
        None => shell.default_path()?,
    };

    if !history_path.exists() {
        bail!(
            "history file not found: {}\n\nExpected location for {} history.",
            history_path.display(),
            shell.name()
        );
    }

    println!();
    println!("  Source: {}", history_path.display());
    println!("  Shell:  {}", shell.name());
    println!();

    // ── Parse ────────────────────────────────────────────────────────────
    println!("  Parsing history...");
    let start = Instant::now();
    let raw_commands = parse_history(&shell, &history_path)?;
    let total_entries = raw_commands.len();

    // ── Filter through ingestion rules ───────────────────────────────────
    let filtered: Vec<&str> = raw_commands
        .iter()
        .map(|s| s.as_str())
        .filter(|cmd| ggnmem_db::should_ingest(cmd))
        .collect();
    let filtered_count = total_entries - filtered.len();

    println!("  Found:    {total_entries:>8} entries");
    println!("  Filtered: {filtered_count:>8} (shell noise, secrets)");
    println!();

    // ── Preview mode ─────────────────────────────────────────────────────
    if preview {
        let sample_count = filtered.len().min(20);
        println!("  Preview (first {sample_count} commands):");
        println!("  ─────────────────────────────────");
        for cmd in filtered.iter().take(sample_count) {
            println!("    {cmd}");
        }
        println!();
        println!("  Total importable: {} commands", filtered.len());
        println!("  Run without --preview to import.");
        return Ok(());
    }

    // ── Dry-run mode ─────────────────────────────────────────────────────
    if dry_run {
        // Open DB read-only to count existing dupes.
        let db_path = super::default_db_path();
        if db_path.exists() {
            let db = Database::open(&DatabaseConfig::new(db_path))
                .context("open database for dedup check")?;
            let existing = db
                .list_all_content_hashes()
                .context("load existing hashes")?;

            let mut would_import = 0u64;
            let mut would_skip = 0u64;
            for cmd in &filtered {
                let hash = ggnmem_db::hash::content_hash(cmd, IMPORT_CWD);
                if existing.contains(&hash) {
                    would_skip += 1;
                } else {
                    would_import += 1;
                }
            }

            println!("  [dry-run] Would import:  {would_import:>8} commands");
            println!("  [dry-run] Would skip:    {would_skip:>8} (duplicates)");
        } else {
            println!("  [dry-run] Would import:  {:>8} commands", filtered.len());
            println!("  [dry-run] Would skip:    {:>8} (no database yet)", 0);
        }
        println!();
        println!("  Run without --dry-run to import.");
        return Ok(());
    }

    // ── Import ───────────────────────────────────────────────────────────
    println!("  Importing...");

    let db_path = super::default_db_path();

    // Ensure data directory exists.
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).context("create database directory")?;
    }

    let db = Database::open(&DatabaseConfig::new(db_path)).context("open database")?;

    // Pre-load existing hashes for fast dedup.
    let existing_hashes = db
        .list_all_content_hashes()
        .context("load existing content hashes")?;

    // Create an import session.
    let now_ms = now_epoch_ms();
    let session_id = SessionId::from_storage(format!("import-{}-{}", shell.name(), now_ms));

    let session = ggnmem_db::NewSession {
        id: session_id.clone(),
        os_context: "linux".to_owned(),
        hostname: hostname(),
        shell: Some(shell.name().to_owned()),
        started_at_ms: now_ms,
    };
    db.insert_session(&session)
        .context("create import session")?;

    // Build NewCommand structs and import in batches.
    let mut imported = 0u64;
    let mut skipped_dupes = 0u64;
    let mut failed = 0u64;
    let batch_size = 1000;

    let mut batch: Vec<NewCommand> = Vec::with_capacity(batch_size);

    for (i, cmd_text) in filtered.iter().enumerate() {
        // Assign synthetic timestamps: spread across the past, ending at now.
        let ts = now_ms - ((filtered.len() - i) as i64 * 1000);

        batch.push(NewCommand {
            id: CommandId::from_storage(uuid::Uuid::new_v4().to_string()),
            session_id: session_id.clone(),
            command: cmd_text.to_string(),
            cwd: IMPORT_CWD.to_owned(),
            exit_code: None,
            duration_ms: None,
            started_at_ms: Some(ts),
            completed_at_ms: ts,
        });

        if batch.len() >= batch_size {
            match db.insert_command_batch(&batch, &existing_hashes) {
                Ok(n) => {
                    imported += n;
                    skipped_dupes += batch.len() as u64 - n;
                }
                Err(e) => {
                    eprintln!("  warning: batch insert failed: {e}");
                    failed += batch.len() as u64;
                }
            }
            batch.clear();
        }
    }

    // Flush remaining batch.
    if !batch.is_empty() {
        match db.insert_command_batch(&batch, &existing_hashes) {
            Ok(n) => {
                imported += n;
                skipped_dupes += batch.len() as u64 - n;
            }
            Err(e) => {
                eprintln!("  warning: batch insert failed: {e}");
                failed += batch.len() as u64;
            }
        }
    }

    let elapsed = start.elapsed();

    println!("  Imported:          {imported:>8} commands");
    println!("  Skipped (dupes):   {skipped_dupes:>8}");
    println!("  Failed:            {failed:>8}");
    println!();
    println!("  Duration: {:.1}s", elapsed.as_secs_f64());
    println!();
    println!("  Run `ggnmem search docker` to search your imported history.");

    Ok(())
}

// ─── Constants ───────────────────────────────────────────────────────────────

/// CWD for imported commands (history files don't record working directory).
const IMPORT_CWD: &str = "imported";

// ─── Shell types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
enum Shell {
    Bash,
    Zsh,
    Fish,
}

impl Shell {
    fn name(self) -> &'static str {
        match self {
            Shell::Bash => "bash",
            Shell::Zsh => "zsh",
            Shell::Fish => "fish",
        }
    }

    fn default_path(self) -> Result<PathBuf> {
        let home = home_dir()?;
        Ok(match self {
            Shell::Bash => home.join(".bash_history"),
            Shell::Zsh => home.join(".zsh_history"),
            Shell::Fish => home
                .join(".local")
                .join("share")
                .join("fish")
                .join("fish_history"),
        })
    }
}

// ─── Shell auto-detection ────────────────────────────────────────────────────

fn detect_shell() -> Result<Shell> {
    // Check $SHELL environment variable first.
    if let Ok(shell_var) = std::env::var("SHELL") {
        let shell_lower = shell_var.to_lowercase();
        if shell_lower.contains("zsh") {
            println!("  Detected shell: zsh (from $SHELL)");
            return Ok(Shell::Zsh);
        }
        if shell_lower.contains("fish") {
            println!("  Detected shell: fish (from $SHELL)");
            return Ok(Shell::Fish);
        }
        if shell_lower.contains("bash") {
            println!("  Detected shell: bash (from $SHELL)");
            return Ok(Shell::Bash);
        }
    }

    // Fallback: check which history files exist.
    let home = home_dir()?;
    if home.join(".zsh_history").exists() {
        println!("  Detected shell: zsh (found ~/.zsh_history)");
        return Ok(Shell::Zsh);
    }
    if home
        .join(".local")
        .join("share")
        .join("fish")
        .join("fish_history")
        .exists()
    {
        println!("  Detected shell: fish (found fish_history)");
        return Ok(Shell::Fish);
    }
    if home.join(".bash_history").exists() {
        println!("  Detected shell: bash (found ~/.bash_history)");
        return Ok(Shell::Bash);
    }

    bail!("could not detect shell — no history files found.\n\nSpecify explicitly:\n  ggnmem import bash\n  ggnmem import zsh\n  ggnmem import fish");
}

// ─── History parsers ─────────────────────────────────────────────────────────

fn parse_history(shell: &Shell, path: &PathBuf) -> Result<Vec<String>> {
    match shell {
        Shell::Bash => parse_bash_history(path),
        Shell::Zsh => parse_zsh_history(path),
        Shell::Fish => parse_fish_history(path),
    }
}

/// Parse Bash history: one command per line.
fn parse_bash_history(path: &PathBuf) -> Result<Vec<String>> {
    let file = std::fs::File::open(path).context("open bash history")?;
    let reader = BufReader::new(file);
    let mut commands = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue, // skip corrupted lines
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Bash sometimes prefixes lines with timestamps: #1234567890
        if trimmed.starts_with('#') {
            continue;
        }
        commands.push(trimmed.to_owned());
    }

    Ok(commands)
}

/// Parse Zsh history: handles both plain format and extended format.
///
/// Extended format: `: timestamp:duration;command`
/// Plain format: one command per line.
/// Multi-line entries: lines ending with `\` continue on the next line.
fn parse_zsh_history(path: &PathBuf) -> Result<Vec<String>> {
    let file = std::fs::File::open(path).context("open zsh history")?;
    let reader = BufReader::new(file);
    let mut commands = Vec::new();
    let mut continuation = String::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        // Handle multi-line entries (lines ending with \).
        if line.ends_with('\\') {
            let without_backslash = &line[..line.len() - 1];
            if continuation.is_empty() {
                // Start of a multi-line entry.
                // Parse the first line which may have the extended format prefix.
                let cmd_part = strip_zsh_extended_prefix(without_backslash);
                continuation.push_str(cmd_part);
            } else {
                continuation.push('\n');
                continuation.push_str(without_backslash);
            }
            continue;
        }

        // End of a multi-line entry or a single-line entry.
        let full_line = if continuation.is_empty() {
            line.clone()
        } else {
            continuation.push('\n');
            continuation.push_str(&line);
            let result = continuation.clone();
            continuation.clear();
            result
        };

        let cmd = strip_zsh_extended_prefix(&full_line);
        let trimmed = cmd.trim();
        if !trimmed.is_empty() {
            commands.push(trimmed.to_owned());
        }
    }

    // Handle trailing continuation.
    if !continuation.is_empty() {
        let trimmed = continuation.trim();
        if !trimmed.is_empty() {
            commands.push(trimmed.to_owned());
        }
    }

    Ok(commands)
}

/// Strip the zsh extended history prefix `: timestamp:duration;`.
fn strip_zsh_extended_prefix(line: &str) -> &str {
    // Extended format: `: 1234567890:0;actual command here`
    if line.starts_with(": ") {
        if let Some(semi_pos) = line.find(';') {
            return &line[semi_pos + 1..];
        }
    }
    line
}

/// Parse Fish history: YAML-like format.
///
/// Format:
/// ```text
/// - cmd: git status
///   when: 1234567890
/// - cmd: docker ps
///   when: 1234567891
/// ```
fn parse_fish_history(path: &PathBuf) -> Result<Vec<String>> {
    let file = std::fs::File::open(path).context("open fish history")?;
    let reader = BufReader::new(file);
    let mut commands = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let trimmed = line.trim();

        // Fish history entries start with "- cmd: "
        if let Some(cmd) = trimmed.strip_prefix("- cmd: ") {
            let cmd = cmd.trim();
            if !cmd.is_empty() {
                // Fish escapes newlines as \n in the cmd line.
                let unescaped = cmd.replace("\\n", "\n");
                // Take only the first line of multi-line commands.
                let first_line = unescaped.lines().next().unwrap_or(cmd).trim();
                if !first_line.is_empty() {
                    commands.push(first_line.to_owned());
                }
            }
        }
    }

    Ok(commands)
}

// ─── Doctor integration ──────────────────────────────────────────────────────

/// Print history import status for `ggnmem doctor`.
pub fn doctor_history_status() {
    println!();
    println!("history import ... available");

    let home = match home_dir() {
        Ok(h) => h,
        Err(_) => {
            println!("  (could not determine home directory)");
            return;
        }
    };

    // Bash.
    let bash_path = home.join(".bash_history");
    print!("  bash history  ... ");
    if bash_path.exists() {
        let size = std::fs::metadata(&bash_path).map(|m| m.len()).unwrap_or(0);
        let lines = count_file_lines(&bash_path).unwrap_or(0);
        println!(
            "✓ {} ({}, ~{} entries)",
            bash_path.display(),
            format_size(size),
            lines
        );
    } else {
        println!("✗ not found");
    }

    // Zsh.
    let zsh_path = home.join(".zsh_history");
    print!("  zsh history   ... ");
    if zsh_path.exists() {
        let size = std::fs::metadata(&zsh_path).map(|m| m.len()).unwrap_or(0);
        let lines = count_file_lines(&zsh_path).unwrap_or(0);
        println!(
            "✓ {} ({}, ~{} entries)",
            zsh_path.display(),
            format_size(size),
            lines
        );
    } else {
        println!("✗ not found");
    }

    // Fish.
    let fish_path = home
        .join(".local")
        .join("share")
        .join("fish")
        .join("fish_history");
    print!("  fish history  ... ");
    if fish_path.exists() {
        let size = std::fs::metadata(&fish_path).map(|m| m.len()).unwrap_or(0);
        let lines = count_file_lines(&fish_path).unwrap_or(0);
        println!(
            "✓ {} ({}, ~{} entries)",
            fish_path.display(),
            format_size(size),
            lines
        );
    } else {
        println!("✗ not found");
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn home_dir() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME environment variable not set")
}

fn hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("HOST"))
        .unwrap_or_else(|_| "imported".to_owned())
}

fn now_epoch_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn has_flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

fn parse_named_arg(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn count_file_lines(path: &PathBuf) -> Result<usize> {
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);
    Ok(reader.lines().count())
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

// ─── Help ────────────────────────────────────────────────────────────────────

fn print_import_help() {
    println!("ggnmem import — import shell history into the ggnmem database");
    println!();
    println!("usage:");
    println!("  ggnmem import auto            Auto-detect shell and import history");
    println!("  ggnmem import bash            Import from ~/.bash_history");
    println!("  ggnmem import zsh             Import from ~/.zsh_history");
    println!("  ggnmem import fish            Import from ~/.local/share/fish/fish_history");
    println!();
    println!("options:");
    println!("  --dry-run        Show counts without modifying the database");
    println!("  --preview        Show a sample of commands before importing");
    println!("  --file <path>    Import from a custom file path");
    println!("  --help, -h       Show this help message");
    println!();
    println!("examples:");
    println!("  ggnmem import auto                  Auto-detect and import");
    println!("  ggnmem import bash --dry-run         Preview bash import without changes");
    println!("  ggnmem import zsh --preview          Show sample zsh commands");
    println!("  ggnmem import bash --file /tmp/hist  Import from custom file");
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_bash_simple() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp.as_file(), "git status").unwrap();
        writeln!(tmp.as_file(), "docker ps").unwrap();
        writeln!(tmp.as_file(), "").unwrap();
        writeln!(tmp.as_file(), "cargo build --release").unwrap();

        let path = tmp.path().to_path_buf();
        let cmds = parse_bash_history(&path).unwrap();
        assert_eq!(
            cmds,
            vec!["git status", "docker ps", "cargo build --release"]
        );
    }

    #[test]
    fn parse_bash_with_timestamps() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp.as_file(), "#1234567890").unwrap();
        writeln!(tmp.as_file(), "git commit -m 'test'").unwrap();
        writeln!(tmp.as_file(), "#1234567891").unwrap();
        writeln!(tmp.as_file(), "npm install").unwrap();

        let path = tmp.path().to_path_buf();
        let cmds = parse_bash_history(&path).unwrap();
        assert_eq!(cmds, vec!["git commit -m 'test'", "npm install"]);
    }

    #[test]
    fn parse_zsh_plain() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp.as_file(), "git status").unwrap();
        writeln!(tmp.as_file(), "docker ps").unwrap();

        let path = tmp.path().to_path_buf();
        let cmds = parse_zsh_history(&path).unwrap();
        assert_eq!(cmds, vec!["git status", "docker ps"]);
    }

    #[test]
    fn parse_zsh_extended() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp.as_file(), ": 1234567890:0;git status").unwrap();
        writeln!(tmp.as_file(), ": 1234567891:5;docker compose up -d").unwrap();

        let path = tmp.path().to_path_buf();
        let cmds = parse_zsh_history(&path).unwrap();
        assert_eq!(cmds, vec!["git status", "docker compose up -d"]);
    }

    #[test]
    fn parse_zsh_multiline() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp.as_file(), ": 1234567890:0;echo hello \\").unwrap();
        writeln!(tmp.as_file(), "world").unwrap();
        writeln!(tmp.as_file(), ": 1234567891:0;git status").unwrap();

        let path = tmp.path().to_path_buf();
        let cmds = parse_zsh_history(&path).unwrap();
        assert_eq!(cmds.len(), 2);
        assert!(cmds[0].contains("echo hello"));
        assert!(cmds[0].contains("world"));
        assert_eq!(cmds[1], "git status");
    }

    #[test]
    fn parse_fish_format() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp.as_file(), "- cmd: git status").unwrap();
        writeln!(tmp.as_file(), "  when: 1234567890").unwrap();
        writeln!(tmp.as_file(), "- cmd: docker ps").unwrap();
        writeln!(tmp.as_file(), "  when: 1234567891").unwrap();

        let path = tmp.path().to_path_buf();
        let cmds = parse_fish_history(&path).unwrap();
        assert_eq!(cmds, vec!["git status", "docker ps"]);
    }

    #[test]
    fn strip_zsh_prefix() {
        assert_eq!(
            strip_zsh_extended_prefix(": 123:0;git status"),
            "git status"
        );
        assert_eq!(strip_zsh_extended_prefix("plain command"), "plain command");
        assert_eq!(
            strip_zsh_extended_prefix(": no-semicolon"),
            ": no-semicolon"
        );
    }
}
