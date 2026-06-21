# Installation

Complete installation guide for **ggnmem** — the Semantic Terminal Memory Engine.

---

## Requirements

### Supported Platforms

| Platform | Architecture | Status |
|----------|-------------|--------|
| Linux | x86_64 | ✅ Supported |
| Linux | aarch64 (ARM64) | ✅ Supported |
| WSL | x86_64 | ✅ Supported |
| WSL | aarch64 | ✅ Supported |
| Windows | x86_64 | ✅ Supported |

### Prerequisites

**Linux / WSL:**

- **Bash** or **Zsh** shell
- `curl` (for the one-line installer)
- No build tools or Rust toolchain required for pre-built releases
- ~100 MB disk space (with AI model)

**Windows:**

- **PowerShell** 5.1 or later (built into Windows 10+)
- Internet connection (for downloading release from GitHub)
- ~100 MB disk space (with AI model)


---

## One-Line Installer

The fastest way to install or upgrade ggnmem:

```bash
curl -fsSL https://raw.githubusercontent.com/gagansokhal-coder/Terminal_helper/main/scripts/install-online.sh | bash
```

### What happens internally

The bootstrap script performs the following steps automatically:

1. **Detects your architecture** — identifies x86_64 or aarch64 and selects the correct release bundle
2. **Downloads the release tarball** — fetches the latest release from GitHub Releases to `/tmp`
3. **Verifies checksums** — validates SHA-256 checksums to ensure binary integrity
4. **Extracts the bundle** — unpacks binaries and support files to a temporary staging directory
5. **Installs binaries** — copies `ggnmem` and `ggnmem-daemon` to `~/.local/bin/`
6. **Creates configuration** — generates a default `config.toml` at `~/.config/ggnmem/`
7. **Sets up shell hooks** — adds `Ctrl+R` integration to your `~/.bashrc` and/or `~/.zshrc`
8. **Verifies installation** — runs `ggnmem version` to confirm everything works
9. **Cleans up** — removes temporary files from `/tmp`

> **Upgrading?** Running the installer on an existing installation performs a safe in-place upgrade. Your configuration, database, and AI models are preserved.

---

## Windows Installation

Install ggnmem on Windows natively with PowerShell:

```powershell
irm https://ggnmem.mytechy.in/install.ps1 | iex
```

### What happens internally

The PowerShell installer performs the following steps automatically:

1. **Detects your environment** — verifies Windows, PowerShell version, and architecture
2. **Fetches the latest release** — queries the GitHub API for the newest release
3. **Downloads the release ZIP** — fetches `ggnmem-windows-x86_64.zip` and `checksums.txt`
4. **Verifies SHA256 checksum** — validates integrity before extraction
5. **Extracts the bundle** — unpacks binaries to a temporary staging directory
6. **Stops existing daemon** — if upgrading, safely stops the running daemon
7. **Backs up existing binaries** — creates `.old` backups before replacing
8. **Installs binaries** — copies `ggnmem.exe` and `ggnmem-daemon.exe` to `%LOCALAPPDATA%\ggnmem\bin\`
9. **Configures PATH** — adds the bin directory to User PATH
10. **Creates configuration** — generates a default `config.toml` at `%APPDATA%\ggnmem\`
11. **Starts daemon** — launches the background daemon
12. **Verifies installation** — runs `ggnmem version` and `ggnmem doctor`
13. **Cleans up** — removes temporary files

### Windows Directory Layout

| Path | Purpose |
|------|---------|
| `%LOCALAPPDATA%\ggnmem\bin\ggnmem.exe` | CLI binary |
| `%LOCALAPPDATA%\ggnmem\bin\ggnmem-daemon.exe` | Background daemon |
| `%LOCALAPPDATA%\ggnmem\data\ggnmem.db` | Command database |
| `%LOCALAPPDATA%\ggnmem\models\` | AI embedding models |
| `%LOCALAPPDATA%\ggnmem\logs\` | Runtime & install logs |
| `%LOCALAPPDATA%\ggnmem\VERSION` | Installed version metadata |
| `%APPDATA%\ggnmem\config.toml` | Configuration |

### Upgrade Behavior

Running the installer on an existing installation:

1. Stops the running daemon
2. Backs up existing binaries (`.old` suffix)
3. Replaces binaries with the new version
4. Preserves database, models, and configuration
5. Restarts the daemon
6. Verifies the new installation

### Rollback

If installation fails mid-flight:

- Previous binaries are automatically restored from backups
- User data (database, config, models) is never modified
- Recovery instructions are printed

> **Upgrading?** Your configuration, database, and AI models are always preserved during upgrades. The installer creates backups and rolls back automatically if anything goes wrong.

---

## Manual Installation

If you prefer to install manually or are on a restricted system without `curl`:

### 1. Download the release bundle

Download the latest release tarball from [GitHub Releases](https://github.com/gagansokhal-coder/Terminal_helper/releases):

```bash
# x86_64
wget https://github.com/gagansokhal-coder/Terminal_helper/releases/latest/download/ggnmem-linux-x86_64.tar.gz

# aarch64
wget https://github.com/gagansokhal-coder/Terminal_helper/releases/latest/download/ggnmem-linux-aarch64.tar.gz
```

### 2. Extract and install

```bash
tar xzf ggnmem-linux-x86_64.tar.gz
bash install.sh
```

The `install.sh` script handles binary placement, configuration, and shell hook setup — the same steps as the one-line installer.

### 3. Build from source

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

---

## Verify Installation

After installing, open a **new terminal** and run:

```bash
# Check installed version
ggnmem version

# Run a full health check
ggnmem doctor
```

Expected output from `ggnmem doctor`:

```
  ✅ Binary:    ~/.local/bin/ggnmem (v0.3.7-alpha)
  ✅ Daemon:    ~/.local/bin/ggnmem-daemon
  ✅ Config:    ~/.config/ggnmem/config.toml
  ✅ Database:  ~/.local/share/ggnmem/ggnmem.db
  ✅ Shell:     bash hook installed
  ✅ PATH:      ~/.local/bin is in PATH
```

---

## Start Daemon

ggnmem uses a lightweight background daemon to capture commands and serve search queries:

```bash
# Start the daemon
ggnmem start

# Check daemon status
ggnmem status
```

The daemon runs in the background, uses < 50 MB of memory, and communicates with the CLI via Unix domain sockets.

### Daemon Management

```bash
ggnmem start       # Start the daemon
ggnmem stop        # Stop the daemon
ggnmem restart     # Restart the daemon
ggnmem status      # Check if the daemon is running
ggnmem logs        # View daemon log output
```

---

## First Search

Once the daemon is running and you've used your terminal for a bit (or imported your history), try searching:

```bash
# Keyword search
ggnmem search docker

# Natural-language search (requires AI setup)
ggnmem search "show running containers"

# Interactive TUI (Ctrl+R replacement)
ggnmem ui
```

### Import Existing History

Start with a populated database by importing your shell history:

```bash
# Auto-detect your shell and import
ggnmem import auto

# Or specify explicitly
ggnmem import bash
ggnmem import zsh
ggnmem import fish

# Preview before importing
ggnmem import bash --preview
ggnmem import bash --dry-run
```

---

## AI Setup

ggnmem works out of the box with fast keyword search (FTS5). To enable AI-powered **semantic search**, set up a local embedding model:

```bash
# Interactive setup wizard (recommended)
ggnmem ai setup

# Or manually:
ggnmem ai install          # Downloads a lightweight embedding model (~30 MB)
ggnmem ai status           # Verify AI is active and model is loaded
```

### Available Models

| Model | Size | Speed | Quality |
|-------|------|-------|---------|
| MiniLM-L6-v2 | ~30 MB | Fast | Good |
| BGE-Small-EN | ~50 MB | Medium | Better |

### Model Management

```bash
ggnmem ai models           # List available models
ggnmem ai use <model>      # Switch active model
ggnmem ai benchmark        # Benchmark installed models
ggnmem ai reindex          # Rebuild all embeddings (after model switch)
ggnmem ai doctor           # Check model health
ggnmem ai verify-model     # Verify model loads correctly
```

All models run **locally via ONNX Runtime** — no API keys, no cloud calls, no internet required after the initial download.

---

## Updating

### Self-Update (Recommended)

```bash
ggnmem self-update
```

This command automatically checks GitHub Releases for the latest version, downloads the correct binary for your architecture, verifies checksums, and performs a safe in-place upgrade.

### Re-run the Installer

```bash
curl -fsSL https://raw.githubusercontent.com/gagansokhal-coder/Terminal_helper/main/scripts/install-online.sh | bash
```

### Manual Upgrade

```bash
# Download and extract the new release
tar xzf ggnmem-linux-x86_64.tar.gz

# Option A: Use the upgrade command
ggnmem upgrade --bundle ./

# Option B: Use the install script (handles stop/start)
bash install.sh

# Option C: Replace binaries manually
ggnmem stop
cp ggnmem ~/.local/bin/ggnmem
cp ggnmem-daemon ~/.local/bin/ggnmem-daemon
chmod +x ~/.local/bin/ggnmem ~/.local/bin/ggnmem-daemon
ggnmem start
ggnmem version
```

> **Safe upgrades:** Your configuration (`config.toml`), command database (`ggnmem.db`), and AI models are always preserved during upgrades.

---

## Uninstall

```bash
# Remove binaries and shell hooks (keeps your database and config)
ggnmem uninstall

# Remove everything, including database, config, and models
ggnmem uninstall --full
```

### What gets removed

| `ggnmem uninstall` | `ggnmem uninstall --full` |
|---------------------|---------------------------|
| `~/.local/bin/ggnmem` | ✅ Removed |
| `~/.local/bin/ggnmem-daemon` | ✅ Removed |
| Shell hooks in `~/.bashrc` / `~/.zshrc` | ✅ Removed |
| `~/.config/ggnmem/` | ❌ Kept | ✅ Removed |
| `~/.local/share/ggnmem/` (database + models) | ❌ Kept | ✅ Removed |
| `~/.local/state/ggnmem/` (logs, PID) | ✅ Removed |

---

## Troubleshooting

### PATH Issues

**Symptom:** `ggnmem: command not found`

**Fix:** Add `~/.local/bin` to your PATH:

```bash
# Bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Zsh
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### Daemon Issues

**Symptom:** Daemon won't start, or commands aren't being captured.

```bash
# Check daemon status
ggnmem status

# Check for stale PID file
ls -la ~/.local/state/ggnmem/daemon.pid
# If the file exists but daemon isn't running:
rm ~/.local/state/ggnmem/daemon.pid

# View daemon logs
ggnmem logs

# Run full diagnostics
ggnmem doctor
```

**Symptom:** `daemon protocol mismatch` error after upgrading.

```bash
# Restart the daemon to pick up the new version
ggnmem restart
```

### WSL Issues

- **Daemon runs inside WSL**, not on the Windows host — this is expected
- Store data on the Linux filesystem (`~/`), **not** on `/mnt/c/` — the Windows filesystem is significantly slower
- If using multiple WSL distributions, install ggnmem separately in each one
- The daemon uses Unix domain sockets in `$XDG_RUNTIME_DIR`; ensure this variable is set in your WSL distribution

### Permission Issues

**Symptom:** Permission denied when starting the daemon or accessing the database.

```bash
# Ensure correct ownership of ggnmem directories
chown -R $(whoami) ~/.config/ggnmem ~/.local/share/ggnmem ~/.local/state/ggnmem

# Ensure binaries are executable
chmod +x ~/.local/bin/ggnmem ~/.local/bin/ggnmem-daemon
```

### Ctrl+R Not Working

```bash
# Verify shell hooks are installed
grep "ggnmem init" ~/.bashrc   # or ~/.zshrc

# If not present, reinstall hooks
ggnmem install
source ~/.bashrc   # or ~/.zshrc

# Check that the daemon is running
ggnmem status
```

### Database Corruption

```bash
# Stop daemon
ggnmem stop

# Back up the database
cp ~/.local/share/ggnmem/ggnmem.db ~/.local/share/ggnmem/ggnmem.db.backup

# Remove the corrupted database (will be recreated on start)
rm ~/.local/share/ggnmem/ggnmem.db

# Start fresh
ggnmem start
```

---

## FAQ

### Where is data stored?

All ggnmem data is stored locally in standard XDG directories:

| Data | Path |
|------|------|
| Configuration | `~/.config/ggnmem/config.toml` |
| Command database | `~/.local/share/ggnmem/ggnmem.db` |
| AI models | `~/.local/share/ggnmem/models/` |
| Runtime state & logs | `~/.local/state/ggnmem/` |
| Binaries | `~/.local/bin/ggnmem`, `~/.local/bin/ggnmem-daemon` |

### Where are AI models stored?

Models are downloaded to `~/.local/share/ggnmem/models/`. Each model is a self-contained ONNX file (~30–50 MB).

### How much disk space is needed?

| Component | Size |
|-----------|------|
| Binaries (`ggnmem` + `ggnmem-daemon`) | ~57 MB |
| AI model (MiniLM) | ~30 MB |
| AI model (BGE-Small) | ~50 MB |
| Database (10k commands) | ~5–15 MB |
| **Total (typical)** | **~100 MB** |

### Is internet required?

**Only for installation and model downloads.** After initial setup, ggnmem runs **100% offline**. No internet connection is needed for command capture, search, or AI inference. It works perfectly on air-gapped systems.

### Is my data sent to the cloud?

**No. Never.** ggnmem makes **zero network requests** during normal operation. There is no telemetry, no analytics, no phone-home behavior. Your command history stays on your machine. The source code is fully auditable — you can verify this yourself.

---

## License

[MIT License](LICENSE) — Copyright (c) 2026 ggnmem contributors.
