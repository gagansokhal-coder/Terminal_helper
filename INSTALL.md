# ggnmem Installation Guide

## Prerequisites

- **Linux x86_64** or **aarch64** (ARM64)
- **WSL** (Windows Subsystem for Linux) is fully supported
- **Bash** or **Zsh** shell
- No build tools required for pre-built releases

## Quick Install (Recommended)

The easiest way to install or upgrade `ggnmem` is using the official bootstrap script. This script automatically detects your architecture (x86_64 or ARM64), verifies checksums, extracts the bundle safely, and sets up your shell hooks.

```bash
curl -sSL https://raw.githubusercontent.com/gagansokhal-coder/Terminal_helper/main/scripts/install-online.sh | bash
```

The installer will:
1. Safely download and verify the binary bundle in `/tmp`
2. Copy binaries to `~/.local/bin/`
3. Create default configuration at `~/.config/ggnmem/config.toml`
4. Set up shell integration (Ctrl+R hook) for Bash and Zsh
5. Verify the installation

## Install from Source

Requires [Rust](https://rustup.rs/) 1.76+:

```bash
git clone https://github.com/gagansokhal-coder/Terminal_helper.git
cd Terminal_helper

# Build release binaries
bash scripts/build_release.sh

# Install
cd release && bash install.sh
```

Or install directly with Cargo:

```bash
cargo build --release
cp target/release/ggnmem-cli ~/.local/bin/ggnmem
cp target/release/ggnmem-daemon ~/.local/bin/ggnmem-daemon
chmod +x ~/.local/bin/ggnmem ~/.local/bin/ggnmem-daemon
ggnmem install
```

## Post-Install Setup

After installing, open a new terminal and run:

```bash
# Start the background daemon
ggnmem start

# Verify everything is working
ggnmem doctor

# Check version info
ggnmem version

# Try interactive search
# Press Ctrl+R
```

## Post-Install: Import History

If you already have shell history, import it to start with a populated database:

```bash
# Auto-detect your shell and import history
ggnmem import auto

# Or specify explicitly
ggnmem import bash
ggnmem import zsh
ggnmem import fish
```

Preview before importing:

```bash
ggnmem import bash --preview    # See a sample of commands
ggnmem import bash --dry-run    # See counts without modifying DB
```

After importing, verify your commands are searchable:

```bash
ggnmem search docker
```

## WSL-Specific Notes

ggnmem works fully in WSL (Windows Subsystem for Linux). A few tips:

- **Daemon runs inside WSL**, not on the Windows host
- Shell hooks are added to your WSL `~/.bashrc` or `~/.zshrc`
- Database and config live in the WSL filesystem (`~/.local/share/ggnmem/`)
- Ctrl+R integration works in any WSL terminal (Windows Terminal, etc.)
- If using multiple WSL distributions, install separately in each one

### WSL Performance

- ggnmem stores data on the Linux filesystem, so performance is native
- Avoid storing the database on `/mnt/c/` (Windows filesystem) — it's slower
- The daemon uses Unix domain sockets in `$XDG_RUNTIME_DIR`

## Upgrade Process

### Using `ggnmem upgrade`

The simplest way to upgrade:

```bash
# Download the new release
tar xzf ggnmem-linux-x86_64.tar.gz

# Upgrade (automatically stops/starts daemon, preserves config & DB)
ggnmem upgrade --bundle ./
```

### Using `install.sh`

Running `install.sh` on an existing installation performs an in-place upgrade:

```bash
tar xzf ggnmem-linux-x86_64.tar.gz
bash install.sh
```

The installer will:
- Detect the existing installation and show old/new version
- Stop the daemon before replacing binaries
- Back up existing binaries to `ggnmem.old` / `ggnmem-daemon.old`
- Preserve your configuration and database
- Verify the new binaries work
- Show upgrade summary

### Manual Upgrade

```bash
# Stop the daemon
ggnmem stop

# Replace binaries
cp ggnmem ~/.local/bin/ggnmem
cp ggnmem-daemon ~/.local/bin/ggnmem-daemon
chmod +x ~/.local/bin/ggnmem ~/.local/bin/ggnmem-daemon

# Restart
ggnmem start

# Verify
ggnmem version
ggnmem doctor
```

## Directory Layout

| Path | Purpose |
|------|---------|
| `~/.local/bin/ggnmem` | CLI binary |
| `~/.local/bin/ggnmem-daemon` | Background daemon |
| `~/.config/ggnmem/config.toml` | Configuration |
| `~/.local/share/ggnmem/ggnmem.db` | Command database |
| `~/.local/share/ggnmem/models/` | AI model files |
| `~/.local/state/ggnmem/` | Runtime state (PID, logs) |
| `~/.local/state/ggnmem/logs/daemon.log` | Daemon log file |

## Troubleshooting

### `ggnmem: command not found`

Your `~/.local/bin` directory is not in PATH. Add it:

```bash
# For bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# For zsh
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### Daemon won't start

1. Check if another daemon is already running:
   ```bash
   ggnmem status
   ```

2. Check for stale PID file:
   ```bash
   ls -la ~/.local/state/ggnmem/daemon.pid
   # If the file exists but daemon isn't running, remove it:
   rm ~/.local/state/ggnmem/daemon.pid
   ```

3. Check daemon logs:
   ```bash
   ggnmem logs
   ```

4. Run doctor for full diagnostics:
   ```bash
   ggnmem doctor
   ```

### Protocol mismatch error

This means the CLI and daemon were built from different versions:

```
daemon protocol mismatch: CLI uses IPC protocol v5, daemon uses v4
```

Fix: restart the daemon after upgrading:

```bash
ggnmem restart
```

### Ctrl+R not working

1. Verify shell hooks are installed:
   ```bash
   grep "ggnmem init" ~/.bashrc   # or ~/.zshrc
   ```

2. If not present, run:
   ```bash
   ggnmem install
   source ~/.bashrc   # or ~/.zshrc
   ```

3. Check that the daemon is running:
   ```bash
   ggnmem status
   ```

4. Run doctor to check TUI and shell hook status:
   ```bash
   ggnmem doctor
   ```

### Database corruption

If the database becomes corrupted:

```bash
# Stop daemon
ggnmem stop

# Back up the database
cp ~/.local/share/ggnmem/ggnmem.db ~/.local/share/ggnmem/ggnmem.db.backup

# Remove the database (will be recreated)
rm ~/.local/share/ggnmem/ggnmem.db

# Start fresh
ggnmem start
```

### Large database / Slow performance

```bash
# Check database size
ggnmem db stats

# Optimize the database
ggnmem optimize

# Clean up old entries
ggnmem cleanup --older-than 90

# Remove duplicates
ggnmem cleanup --duplicates
```

## Uninstall

```bash
# Keep database and config
ggnmem uninstall

# Remove everything including database
ggnmem uninstall --full
```

## Verifying a Release

If you're testing a release before distributing:

```bash
bash scripts/test_release.sh ggnmem-linux-x86_64.tar.gz
```

This runs an automated end-to-end verification of the release bundle.

## License

MIT OR Apache-2.0
