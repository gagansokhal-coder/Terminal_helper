<div align="center">

# ggnmem

**Semantic Terminal Memory Engine**

Your shell history, understood — not just stored.

[![CI](https://github.com/gagansokhal-coder/Terminal_helper/actions/workflows/ci.yml/badge.svg)](https://github.com/gagansokhal-coder/Terminal_helper/actions/workflows/ci.yml)
[![Release](https://github.com/gagansokhal-coder/Terminal_helper/actions/workflows/release.yml/badge.svg)](https://github.com/gagansokhal-coder/Terminal_helper/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

</div>

---

## What is ggnmem?

**ggnmem** is a local-first, privacy-focused terminal history intelligence system built in Rust. It captures every command you run, indexes it with both keyword and AI-powered semantic search, and lets you retrieve it instantly — all without sending a single byte to the cloud.

Think of it as `Ctrl+R` on steroids:

- Type `"show running containers"` and find `docker ps` — even if you never typed those exact words.
- Search by intent, not just keystrokes.
- Everything stays on your machine. No accounts, no telemetry, no API keys.

```
$ ggnmem search "check git changes"

  [HYB] git status                          ~/projects/ggnmem    [0]   12ms
  [FTS] git diff --staged                   ~/projects/ggnmem    [0]    8ms
  [SEM] git log --oneline -5               ~/projects/ggnmem    [0]   45ms
```

---

## Features

### 🔍 Hybrid Search Engine
- **FTS5 keyword search** — blazing-fast full-text search with BM25 ranking
- **Semantic vector search** — AI-powered intent matching using local embeddings
- **Reciprocal Rank Fusion** — intelligently merges both result sets for the best answer

### 🧠 Local AI (Zero Cloud)
- Runs **100% offline** — no API keys, no cloud calls, no telemetry
- Lightweight ONNX-based embedding models (MiniLM, BGE)
- Background daemon handles embedding generation without blocking your terminal

### ⚡ Built for Speed
- Shell hook latency: **< 10ms** — you won't notice it
- Search response: **< 100ms** on typical history
- Idle daemon memory: **< 50 MB**

### 🖥️ Interactive TUI
- Full-screen terminal UI bound to `Ctrl+R`
- Cycle search modes with `Ctrl+F`: FTS → Semantic → Hybrid
- Color-coded source badges, latency display, database count in status bar
- `Ctrl+C` to copy selected command, `PgUp/PgDn` for page navigation
- Command preview panel with score, source, exit code, and run count
- Helpful empty-state tips when no results are found

### 🔒 Privacy First
- All data stored locally in SQLite (WAL mode)
- Secret redaction — API keys, tokens, passwords are scrubbed before storage
- Database and IPC sockets locked with strict file permissions
- No network requests, ever

### 📦 Self-Contained
- Single-binary CLI + background daemon
- Shell integration for **Bash** and **Zsh**
- Built-in install, upgrade, uninstall, and health checks
- Automated release pipeline with GitHub Actions

### 📥 Shell History Import
- Import existing Bash, Zsh, and Fish history instantly
- Auto-detect your shell or specify explicitly
- Deduplication — never imports the same command twice
- Preview and dry-run modes for safety
- Handles 100k+ history entries with streaming + batch writes

---

## Installation

### Pre-built Binaries (Recommended)

Download the latest release for your platform from [**GitHub Releases**](https://github.com/gagansokhal-coder/Terminal_helper/releases):

```bash
# Download and extract
tar xzf ggnmem-linux-x86_64.tar.gz

# Install (copies binaries, sets up shell integration)
bash install.sh
```

### Build from Source

Requires [Rust](https://rustup.rs/) 1.76+:

```bash
git clone https://github.com/gagansokhal-coder/Terminal_helper.git
cd Terminal_helper

# Build release binaries
bash scripts/build_release.sh

# Install
cd release && bash install.sh
```

### Supported Platforms

| Platform | Architecture | Status |
|----------|-------------|--------|
| Linux    | x86_64      | ✅ Supported |
| Linux    | aarch64     | ✅ Supported |
| WSL      | x86_64      | ✅ Supported |
| Windows  | —           | 🔜 Planned  |

---

## Quick Start

```bash
# 1. Start the background daemon
ggnmem start

# 2. Verify everything is working
ggnmem doctor

# 3. Open a new terminal and use it normally — commands are captured automatically

# 4. Search your history
ggnmem search "docker compose"

# 5. Interactive search (Ctrl+R replacement)
ggnmem ui

# 6. Import your existing shell history
ggnmem import auto

# 7. View recent commands
ggnmem recent
```

### Shell Integration

After installing, your shell's `Ctrl+R` is upgraded to ggnmem's interactive search. Open a new terminal or source your shell config:

```bash
source ~/.bashrc   # or ~/.zshrc
```

### Useful Commands

```bash
ggnmem search <query>     # Search history (hybrid mode)
ggnmem ui                 # Interactive TUI search
ggnmem recent             # Show recent commands
ggnmem stats              # Database statistics
ggnmem doctor             # Full health check
ggnmem version --verbose  # Detailed version info
ggnmem import auto        # Import shell history
```

---

## AI Setup

ggnmem works out of the box with keyword search. To enable AI-powered semantic search:

```bash
# Interactive setup wizard
ggnmem ai setup

# Or manually:
ggnmem ai enable
ggnmem ai install          # Downloads a lightweight embedding model (~30 MB)
ggnmem ai status           # Verify AI is active
```

### Available Models

| Model | Size | Speed | Quality |
|-------|------|-------|---------|
| MiniLM-L6-v2 | ~30 MB | Fast | Good |
| BGE-Small-EN | ~50 MB | Medium | Better |

```bash
# List models
ggnmem ai models

# Switch active model
ggnmem ai use bge-small-en-v1.5

# Benchmark installed models
ggnmem ai benchmark

# Rebuild all embeddings (after model switch)
ggnmem ai reindex
```

### AI Diagnostics

```bash
ggnmem ai doctor           # Check model health
ggnmem ai verify-model     # Verify model loads and produces embeddings
```

---

## GitHub Releases

Releases are fully automated. Pushing a version tag triggers the CI/CD pipeline:

```bash
# Tag a release
git tag -a v0.4.0-alpha -m "v0.4.0-alpha"
git push origin v0.4.0-alpha
```

The pipeline automatically:
1. Runs all CI checks (fmt, clippy, tests)
2. Verifies tag version matches `Cargo.toml`
3. Cross-compiles for x86_64 and aarch64
4. Verifies binary version matches tag
5. Creates a GitHub Release with tarballs, checksums, and release notes

Pre-release tags (`-alpha`, `-beta`, `-rc`) are marked as prerelease automatically.

See [`.github/workflows/release.yml`](.github/workflows/release.yml) for the full pipeline.

---

## History Import

New to ggnmem? Import your existing shell history so you start with a populated database:

```bash
# Auto-detect your shell and import
ggnmem import auto

# Or specify explicitly
ggnmem import bash
ggnmem import zsh
ggnmem import fish
```

### Options

```bash
# Preview what would be imported (first 20 commands)
ggnmem import bash --preview

# Dry run — show counts without modifying the database
ggnmem import bash --dry-run

# Import from a custom file path
ggnmem import bash --file /path/to/custom_history
```

### Example Output

```text
  Source: /home/user/.bash_history
  Shell:  bash

  Parsing history...
  Found:       12847 entries
  Filtered:     1203 (shell noise, secrets)

  Importing...
  Imported:          9421 commands
  Skipped (dupes):   2223
  Failed:               0

  Duration: 1.2s
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  Shell (Bash / Zsh)                                         │
│  preexec/precmd hooks → ggnmem ingest (< 10ms)              │
└───────────────────────────┬─────────────────────────────────┘
                            │ Unix Domain Socket (IPC)
┌───────────────────────────▼─────────────────────────────────┐
│  ggnmem-daemon (Background)                                  │
│  ┌─────────┐  ┌──────────┐  ┌─────────────┐                │
│  │ Ingestion│  │ Embedding│  │ Search      │                │
│  │ Queue    │→ │ Worker   │  │ FTS + Vector│                │
│  └─────────┘  └──────────┘  └─────────────┘                │
│       │              │              │                        │
│  ┌────▼──────────────▼──────────────▼────┐                  │
│  │         SQLite (WAL mode)             │                  │
│  │  commands │ FTS5 index │ vector store  │                  │
│  └───────────────────────────────────────┘                  │
└─────────────────────────────────────────────────────────────┘
```

### Crate Structure

| Crate | Purpose |
|-------|---------|
| `ggnmem-cli` | CLI binary, shell hooks, TUI, upgrade command |
| `ggnmem-daemon` | Background daemon, IPC server, config, storage |
| `ggnmem-db` | SQLite schema, migrations, FTS5, search engine |
| `ggnmem-ai` | ONNX embeddings, vector store, model management |
| `ggnmem-model` | Data models and protocol definitions |
| `ggnmem-pty` | PTY proxy for terminal overlay |
| `ggnmem-knowledge` | Built-in command knowledge base |

---

## Roadmap

| Milestone | Status |
|-----------|--------|
| Cargo workspace + crate boundaries | ✅ Done |
| SQLite database + FTS5 search | ✅ Done |
| Linux daemon (Unix Domain Sockets) | ✅ Done |
| Bash + Zsh shell capture | ✅ Done |
| Local AI embeddings (ONNX) | ✅ Done |
| Hybrid search (RRF) | ✅ Done |
| Interactive TUI (`Ctrl+R`) | ✅ Done |
| Install / upgrade / uninstall | ✅ Done |
| Knowledge base (ask, explain, learn) | ✅ Done |
| CI/CD + automated releases | ✅ Done |
| Shell history import (Bash, Zsh, Fish) | ✅ Done |
| Enhanced TUI & Ctrl+R experience | ✅ Done |
| Windows PowerShell support | 🔜 Planned |
| Ghost-text autosuggestions | 🔜 Future |
| Fish / Nushell integration | 🔜 Future |

See [docs/roadmap.md](docs/roadmap.md) for the full phased execution plan.

---

## Directory Layout

| Path | Purpose |
|------|---------|
| `~/.local/bin/ggnmem` | CLI binary |
| `~/.local/bin/ggnmem-daemon` | Background daemon |
| `~/.config/ggnmem/config.toml` | Configuration |
| `~/.local/share/ggnmem/ggnmem.db` | Command database |
| `~/.local/share/ggnmem/models/` | AI model files |
| `~/.local/state/ggnmem/` | Runtime state, logs, PID |

---

## Contributing

Contributions are welcome! Please ensure:

1. `cargo fmt --all` passes
2. `cargo clippy --workspace --all-targets -- -D warnings` passes
3. `cargo test --workspace` passes
4. New features include tests

---

## License

This project is licensed under the [MIT License](LICENSE).

```
MIT License — Copyright (c) 2026 ggnmem contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files, to deal in the Software
without restriction, including without limitation the rights to use, copy,
modify, merge, publish, distribute, sublicense, and/or sell copies of the
Software, subject to the conditions in the LICENSE file.
```

---

<div align="center">

**Built with Rust 🦀 · Runs 100% locally · No cloud, no telemetry, no compromise.**

</div>
