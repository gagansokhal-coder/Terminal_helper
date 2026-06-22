//! Install and uninstall flows for ggnmem.
//!
//! `ggnmem install`  — set up config, shell integration, and verify.
//! `ggnmem uninstall` — remove binaries, hooks, config. Preserve DB unless `--full`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

// ─── Directory helpers ───────────────────────────────────────────────────────

fn bin_dir() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        let local_app_data = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .context("LOCALAPPDATA is not set")?;
        Ok(local_app_data.join("ggnmem").join("bin"))
    }

    #[cfg(unix)]
    {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .context("HOME is not set")?;
        Ok(home.join(".local").join("bin"))
    }
}

fn config_dir() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        let app_data = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .context("APPDATA is not set")?;
        Ok(app_data.join("ggnmem"))
    }

    #[cfg(unix)]
    {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .context("HOME is not set")?;
        Ok(home.join(".config").join("ggnmem"))
    }
}

fn data_dir() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        let local_app_data = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .context("LOCALAPPDATA is not set")?;
        Ok(local_app_data.join("ggnmem").join("data"))
    }

    #[cfg(unix)]
    {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .context("HOME is not set")?;
        Ok(home.join(".local").join("share").join("ggnmem"))
    }
}

fn state_dir() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        let local_app_data = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .context("LOCALAPPDATA is not set")?;
        Ok(local_app_data.join("ggnmem"))
    }

    #[cfg(unix)]
    {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .context("HOME is not set")?;
        Ok(home.join(".local").join("state").join("ggnmem"))
    }
}

/// Home directory — only used for Unix shell rc file paths.
#[cfg(unix)]
fn home_dir() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is not set")
}

// ─── Default config ──────────────────────────────────────────────────────────

const DEFAULT_CONFIG: &str = r#"# ggnmem configuration
# See: https://github.com/ggnmem/ggnmem

[features]
capture = true
search = true
tui = true
ai = false

[daemon]
autostart = false

[appearance]
theme = "auto"

[limits]
max_history = 100000
max_memory_mb = 40
max_db_size_mb = 1024

[search]
index_mode = "balanced"

[retention]
retention_days = 365
max_commands = 1000000
auto_cleanup = true

[ai]
ai_enabled = false
embedding_provider = "local"
semantic_search = false
model_name = "all-MiniLM-L6-v2"
"#;

// ─── Install ─────────────────────────────────────────────────────────────────

pub fn install() -> Result<()> {
    println!("ggnmem install");
    println!("═══════════════════════════════════════");
    println!();

    // 1. Create directories.
    let dirs = [
        ("config", config_dir()?),
        ("data", data_dir()?),
        ("state", state_dir()?),
        ("bin", bin_dir()?),
    ];

    for (label, dir) in &dirs {
        if dir.exists() {
            println!("  ✓ {label:>6}  {}", dir.display());
        } else {
            fs::create_dir_all(dir)
                .with_context(|| format!("create {label} directory: {}", dir.display()))?;
            println!("  + {label:>6}  {} (created)", dir.display());
        }
    }
    println!();

    // 2. Write default config.
    let config_path = config_dir()?.join("config.toml");
    if config_path.exists() {
        println!(
            "  ✓ config  {} (exists, not overwriting)",
            config_path.display()
        );
    } else {
        fs::write(&config_path, DEFAULT_CONFIG)
            .with_context(|| format!("write config: {}", config_path.display()))?;
        println!("  + config  {} (created)", config_path.display());
    }
    println!();

    // 3. Shell integration (Unix only).
    #[cfg(unix)]
    {
        let shell = detect_shell();
        match shell.as_deref() {
            Some("zsh") => {
                let rc = home_dir()?.join(".zshrc");
                add_shell_integration(&rc, "zsh")?;
            }
            Some("bash") => {
                let rc = home_dir()?.join(".bashrc");
                add_shell_integration(&rc, "bash")?;
            }
            Some(other) => {
                println!("  ⚠ shell   unsupported shell: {other}");
                println!("           add manually: eval \"$(ggnmem init <shell>)\"");
            }
            None => {
                println!("  ⚠ shell   could not detect shell");
                println!("           add manually: eval \"$(ggnmem init <shell>)\"");
            }
        }
        println!();

        // 4. PATH check.
        let bin = bin_dir()?;
        let path_var = std::env::var("PATH").unwrap_or_default();
        let bin_str = bin.to_string_lossy();
        if path_var.split(':').any(|p| p == bin_str.as_ref()) {
            println!("  ✓ PATH    ~/.local/bin is in PATH");
        } else {
            println!("  ⚠ PATH    ~/.local/bin is NOT in PATH");
            println!("           add to your shell rc:");
            println!("           export PATH=\"$HOME/.local/bin:$PATH\"");
            // Try to add it to the shell rc file.
            if let Some(shell_name) = shell.as_deref() {
                let rc_path = match shell_name {
                    "zsh" => home_dir()?.join(".zshrc"),
                    "bash" => home_dir()?.join(".bashrc"),
                    _ => home_dir()?.join(".profile"),
                };
                add_path_export(&rc_path)?;
            }
        }
        println!();
    }

    #[cfg(windows)]
    {
        println!("  ✓ shell   shell integration not required on Windows");
        println!();

        // PATH check for Windows.
        let bin = bin_dir()?;
        let path_var = std::env::var("PATH").unwrap_or_default();
        let bin_str = bin.to_string_lossy();
        if path_var.split(';').any(|p| p.eq_ignore_ascii_case(&bin_str)) {
            println!("  ✓ PATH    {} is in PATH", bin.display());
        } else {
            println!("  ⚠ PATH    {} is NOT in PATH", bin.display());
            println!("           add it to your system PATH via:");
            println!("           [System Settings] > Environment Variables > PATH");
        }
        println!();
    }

    // 5. Health summary.
    println!("═══════════════════════════════════════");
    println!("  install complete");
    println!();
    println!("  next steps:");
    #[cfg(unix)]
    println!("    1. source your shell rc or open a new terminal");
    #[cfg(windows)]
    println!("    1. open a new terminal if you updated PATH");
    println!("    2. start the daemon:  ggnmem start");
    println!("    3. verify:            ggnmem doctor");
    println!();

    Ok(())
}

// ─── Uninstall ───────────────────────────────────────────────────────────────

pub fn uninstall(args: &[String]) -> Result<()> {
    let full = args.iter().any(|a| a == "--full");

    println!("ggnmem uninstall{}", if full { " --full" } else { "" });
    println!("═══════════════════════════════════════");
    println!();

    // 1. Remove shell integration from rc files (Unix only).
    #[cfg(unix)]
    {
        for rc_name in &[".bashrc", ".zshrc"] {
            let rc_path = home_dir()?.join(rc_name);
            if rc_path.exists() {
                remove_shell_integration(&rc_path)?;
            }
        }
    }

    // 2. Remove binaries.
    let bin = bin_dir()?;
    #[cfg(windows)]
    let binaries = ["ggnmem.exe", "ggnmem-daemon.exe"];
    #[cfg(unix)]
    let binaries = ["ggnmem", "ggnmem-daemon"];
    for binary in &binaries {
        let bin_path = bin.join(binary);
        if bin_path.exists() {
            fs::remove_file(&bin_path)
                .with_context(|| format!("remove binary: {}", bin_path.display()))?;
            println!("  ✗ binary  {} (removed)", bin_path.display());
        } else {
            println!("  - binary  {} (not found)", bin_path.display());
        }
    }

    // 3. Remove config directory.
    let config = config_dir()?;
    if config.exists() {
        fs::remove_dir_all(&config)
            .with_context(|| format!("remove config dir: {}", config.display()))?;
        println!("  ✗ config  {} (removed)", config.display());
    }

    // 4. Remove state directory.
    let state = state_dir()?;
    if state.exists() {
        fs::remove_dir_all(&state)
            .with_context(|| format!("remove state dir: {}", state.display()))?;
        println!("  ✗ state   {} (removed)", state.display());
    }

    // 5. Database.
    let data = data_dir()?;
    if full {
        if data.exists() {
            fs::remove_dir_all(&data)
                .with_context(|| format!("remove data dir: {}", data.display()))?;
            println!("  ✗ data    {} (removed)", data.display());
        }
    } else {
        println!("  ◆ data    {} (preserved)", data.display());
        println!("           use --full to remove database");
    }

    println!();
    println!("═══════════════════════════════════════");
    println!("  uninstall complete");
    println!();

    Ok(())
}

// ─── Shell helpers ───────────────────────────────────────────────────────────

/// Detect the current shell from $SHELL.
#[cfg(unix)]
fn detect_shell() -> Option<String> {
    std::env::var("SHELL").ok().and_then(|s| {
        let name = Path::new(&s).file_name()?.to_str()?.to_owned();
        Some(name)
    })
}

#[cfg(unix)]
const GGNMEM_MARKER: &str = "# ggnmem shell integration";
#[cfg(unix)]
const GGNMEM_MARKER_END: &str = "# end ggnmem";

/// Add shell integration lines to an rc file if not already present.
#[cfg(unix)]
fn add_shell_integration(rc_path: &Path, shell: &str) -> Result<()> {
    if rc_path.exists() {
        let contents =
            fs::read_to_string(rc_path).with_context(|| format!("read {}", rc_path.display()))?;
        if contents.contains("ggnmem init") {
            println!("  ✓ shell   {} (already configured)", rc_path.display());
            return Ok(());
        }
    }

    let integration =
        format!("\n{GGNMEM_MARKER}\neval \"$(ggnmem init {shell})\"\n{GGNMEM_MARKER_END}\n");

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(rc_path)
        .with_context(|| format!("open {} for append", rc_path.display()))?;

    use std::io::Write;
    file.write_all(integration.as_bytes())
        .with_context(|| format!("write shell integration to {}", rc_path.display()))?;

    println!("  + shell   {} (added integration)", rc_path.display());
    Ok(())
}

/// Remove ggnmem integration lines from an rc file.
#[cfg(unix)]
fn remove_shell_integration(rc_path: &Path) -> Result<()> {
    let contents =
        fs::read_to_string(rc_path).with_context(|| format!("read {}", rc_path.display()))?;

    if !contents.contains("ggnmem init") && !contents.contains(GGNMEM_MARKER) {
        return Ok(());
    }

    // Remove the block between markers, or individual eval lines.
    let mut output = String::with_capacity(contents.len());
    let mut in_block = false;

    for line in contents.lines() {
        if line.contains(GGNMEM_MARKER) && !line.contains(GGNMEM_MARKER_END) {
            in_block = true;
            continue;
        }
        if line.contains(GGNMEM_MARKER_END) {
            in_block = false;
            continue;
        }
        if in_block {
            continue;
        }
        // Also remove standalone eval lines that aren't in a block.
        if line.contains("ggnmem init") {
            continue;
        }
        output.push_str(line);
        output.push('\n');
    }

    fs::write(rc_path, &output).with_context(|| format!("write cleaned {}", rc_path.display()))?;

    println!("  ✗ shell   {} (removed integration)", rc_path.display());
    Ok(())
}

/// Add PATH export to rc file if not already present.
#[cfg(unix)]
fn add_path_export(rc_path: &Path) -> Result<()> {
    if rc_path.exists() {
        let contents =
            fs::read_to_string(rc_path).with_context(|| format!("read {}", rc_path.display()))?;
        if contents.contains(".local/bin") {
            return Ok(());
        }
    }

    let export_line = "\n# Added by ggnmem installer\nexport PATH=\"$HOME/.local/bin:$PATH\"\n";

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(rc_path)
        .with_context(|| format!("open {} for append", rc_path.display()))?;

    use std::io::Write;
    file.write_all(export_line.as_bytes())
        .with_context(|| format!("write PATH export to {}", rc_path.display()))?;

    println!("           added PATH export to {}", rc_path.display());
    Ok(())
}
