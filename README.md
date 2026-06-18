<div align="center">

# ggnmem

**Semantic Terminal Memory Engine**

Your shell history, understood — not just stored.

[![CI](https://github.com/gagansokhal-coder/Terminal_helper/actions/workflows/ci.yml/badge.svg)](https://github.com/gagansokhal-coder/Terminal_helper/actions/workflows/ci.yml)
[![Release](https://github.com/gagansokhal-coder/Terminal_helper/actions/workflows/release.yml/badge.svg)](https://github.com/gagansokhal-coder/Terminal_helper/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform: Linux](https://img.shields.io/badge/Platform-Linux%20%7C%20WSL-informational)](INSTALL.md)
[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust%20🦀-orange)](https://www.rust-lang.org/)

*A local-first, privacy-focused terminal history intelligence system built in Rust.<br/>Search your commands by **intent**, not just keystrokes — all without ever leaving your machine.*

[Quick Install](#quick-install) · [Features](#features) · [Example Usage](#example-usage) · [Architecture](#architecture) · [Docs](INSTALL.md)

</div>

---

## Features

- ⚡ **Automatic command capture** — shell hooks silently record every command in < 10ms
- 🔍 **Hybrid search (FTS5 + semantic)** — full-text keyword search *and* AI-powered intent matching, fused with Reciprocal Rank Fusion
- 🧠 **AI-powered embeddings** — lightweight local ONNX models (MiniLM, BGE) generate vector embeddings for semantic understanding
- 🖥️ **Interactive TUI** — full-screen terminal UI bound to `Ctrl+R` with mode cycling, color-coded badges, and preview panes
- 📥 **Command history import** — instantly import existing Bash, Zsh, and Fish history with deduplication and dry-run modes
- 🔄 **Self-update system** — upgrade in place with a single command: `ggnmem self-update`
- 📦 **One-line installer** — `curl | bash` bootstrap that detects architecture, verifies checksums, and configures shell hooks
- 🏠 **Local-first architecture** — all data lives in SQLite on your machine; no servers, no accounts, no cloud
- 🔒 **Privacy focused** — zero network requests, secret redaction, strict file permissions; your commands never leave your system

---

## Why ggnmem?

**The problem:** Every developer has been there. You spent 20 minutes crafting the perfect `ffmpeg` command, a gnarly `awk` pipeline, or a multi-flag `docker` incantation — and three weeks later, you can't remember it. You press `Ctrl+R`, type a half-remembered keyword, scroll through hundreds of irrelevant matches, and eventually give up and Google it again.

Traditional shell history is a flat log. It stores *keystrokes*, not *meaning*. It can't tell you which command "shows running containers" — because it doesn't understand intent.

**The solution:** ggnmem replaces your dumb history file with an intelligent, searchable memory engine:

| You type | ggnmem finds |
|----------|-------------|
| `"show running containers"` | `docker ps` |
| `"check git changes"` | `git status`, `git diff --staged` |
| `"compress a folder"` | `tar -czf archive.tar.gz ./folder` |
| `"find large files"` | `find / -type f -size +100M` |
| `"SSH tunnel to database"` | `ssh -L 5432:localhost:5432 user@bastion` |

It captures every command automatically, indexes it with both keyword and AI-powered semantic search, and retrieves it instantly — all **100% offline**, with **zero cloud dependencies**.

---

## Quick Install

```bash
curl -fsSL https://raw.githubusercontent.com/gagansokhal-coder/Terminal_helper/main/scripts/install-online.sh | bash
```

The installer automatically detects your architecture (x86_64 / aarch64), verifies checksums, installs binaries to `~/.local/bin/`, and sets up shell hooks for Bash and Zsh.

> **Supported platforms:** Linux x86_64 · Linux aarch64 · WSL (Windows Subsystem for Linux)

For manual installation, build-from-source instructions, and WSL-specific notes, see [**INSTALL.md**](INSTALL.md).

---

## First Run

```bash
# 1. Check system health
ggnmem doctor

# 2. Start the background daemon
ggnmem start

# 3. Search your history
ggnmem search docker

# 4. Ask in natural language
ggnmem ask "show running containers"
```

After starting the daemon, open a **new terminal** — commands are captured automatically via shell hooks. Press `Ctrl+R` for the interactive search TUI.

---

## Example Usage

```bash
# Keyword search across your history
ggnmem search "kubectl apply"

# Natural-language semantic search
ggnmem search "restart the web server"

# Ask the knowledge base
ggnmem ask "how do I squash git commits?"

# Interactive TUI with mode cycling (FTS → Semantic → Hybrid)
ggnmem ui

# View recent commands
ggnmem recent

# Import your existing shell history
ggnmem import auto

# Check database statistics
ggnmem stats

# Set up local AI embeddings
ggnmem ai setup

# Run a full health check
ggnmem doctor

# Upgrade to the latest release
ggnmem self-update
```

### Search in Action

```
$ ggnmem search "check disk usage"

  [HYB] df -h                                 ~/            [0]   12ms
  [SEM] du -sh /var/log/*                      ~/logs        [0]   38ms
  [FTS] ncdu /home                             ~/            [0]    6ms
```

Results are tagged by source — **FTS** (keyword), **SEM** (semantic), or **HYB** (hybrid fusion) — with latency and exit code displayed inline.

---

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│  Shell (Bash / Zsh)                                              │
│  preexec / precmd hooks → ggnmem ingest (< 10ms)                 │
└──────────────────────────────┬───────────────────────────────────┘
                               │ Unix Domain Socket (IPC)
┌──────────────────────────────▼───────────────────────────────────┐
│  ggnmem-daemon (Background Process)                               │
│                                                                   │
│  ┌───────────┐   ┌────────────┐   ┌──────────────────┐           │
│  │ Ingestion  │   │ Embedding  │   │ Search Engine    │           │
│  │ Queue      │──▶│ Worker     │   │ FTS5 + Vectors   │           │
│  └───────────┘   └────────────┘   └──────────────────┘           │
│        │               │                   │                      │
│  ┌─────▼───────────────▼───────────────────▼──────┐              │
│  │              SQLite (WAL mode)                  │              │
│  │   commands  │  FTS5 index  │  vector store      │              │
│  └────────────────────────────────────────────────┘              │
└──────────────────────────────────────────────────────────────────┘
                               │
                               ▼
                        Search / AI
                  (Hybrid results via RRF)
```

### Crate Structure

| Crate | Purpose |
|-------|---------|
| `ggnmem-cli` | CLI binary, shell hooks, TUI, self-update |
| `ggnmem-daemon` | Background daemon, IPC server, configuration |
| `ggnmem-db` | SQLite schema, migrations, FTS5 search engine |
| `ggnmem-ai` | ONNX embeddings, vector store, model management |
| `ggnmem-model` | Data models and protocol definitions |
| `ggnmem-pty` | PTY proxy for terminal overlay |
| `ggnmem-knowledge` | Built-in command knowledge base |

### Performance

| Metric | Target |
|--------|--------|
| Shell hook latency | < 10ms |
| Search response | < 100ms |
| Daemon idle memory | < 50 MB |

---

## AI Features

ggnmem goes beyond keyword matching. Its AI subsystem converts commands into dense vector embeddings so you can **search by meaning, not just text**.

### Semantic Search

When you search for `"show running containers"`, ggnmem doesn't just look for those exact words — it computes a vector embedding of your query and finds commands whose *meaning* is closest, even if the words are completely different (like `docker ps`).

### Local Embeddings

All AI inference runs **locally on your CPU** using lightweight [ONNX Runtime](https://onnxruntime.ai/) models:

| Model | Size | Speed | Quality |
|-------|------|-------|---------|
| MiniLM-L6-v2 | ~30 MB | Fast | Good |
| BGE-Small-EN | ~50 MB | Medium | Better |

```bash
ggnmem ai setup          # Interactive setup wizard
ggnmem ai install         # Download embedding model (~30 MB)
ggnmem ai status          # Check AI subsystem health
ggnmem ai models          # List available models
ggnmem ai benchmark       # Benchmark model performance
```

### No Cloud Dependency

- Models are downloaded once and run entirely offline
- No API keys, no subscriptions, no rate limits
- No data ever leaves your machine
- Works on air-gapped systems after initial model download

---

## Installation

For detailed installation instructions — including manual install, build from source, WSL-specific notes, and troubleshooting — see the full [**Installation Guide (INSTALL.md)**](INSTALL.md).

---

## Updating

```bash
# One-command self-update (downloads latest release automatically)
ggnmem self-update
```

The self-update system:
1. Checks GitHub Releases for the latest version
2. Downloads the correct binary for your architecture
3. Verifies checksums
4. Stops the daemon, replaces binaries, and restarts
5. Preserves your configuration and database

You can also upgrade manually:

```bash
# Re-run the online installer
curl -fsSL https://raw.githubusercontent.com/gagansokhal-coder/Terminal_helper/main/scripts/install-online.sh | bash

# Or from a release tarball
tar xzf ggnmem-linux-x86_64.tar.gz && bash install.sh
```

---

## Privacy

ggnmem is built on a **local-first philosophy**. Your terminal history is deeply personal — it contains project names, server addresses, file paths, and sometimes secrets. We believe that data should never leave your control.

### Principles

- **Zero network requests** — ggnmem never phones home, never checks for analytics, never uploads anything
- **No accounts or telemetry** — there is no signup, no tracking, no usage metrics
- **Secret redaction** — API keys, tokens, and passwords are automatically scrubbed before storage
- **Strict file permissions** — database files and IPC sockets are locked to your user
- **Fully auditable** — ggnmem is open source; read every line of code yourself

### Data Storage

All data lives on your machine in standard XDG directories:

| Data | Location |
|------|----------|
| Configuration | `~/.config/ggnmem/config.toml` |
| Command database | `~/.local/share/ggnmem/ggnmem.db` |
| AI models | `~/.local/share/ggnmem/models/` |
| Runtime state | `~/.local/state/ggnmem/` |
| Binaries | `~/.local/bin/ggnmem`, `~/.local/bin/ggnmem-daemon` |

---

## Roadmap

### Completed

| Milestone | Status |
|-----------|--------|
| Cargo workspace + crate boundaries | ✅ Done |
| SQLite database + FTS5 search | ✅ Done |
| Linux daemon (Unix Domain Sockets) | ✅ Done |
| Bash + Zsh shell capture | ✅ Done |
| Local AI embeddings (ONNX) | ✅ Done |
| Hybrid search (Reciprocal Rank Fusion) | ✅ Done |
| Interactive TUI (`Ctrl+R` replacement) | ✅ Done |
| Install / upgrade / uninstall system | ✅ Done |
| Knowledge base (`ask`, `explain`, `learn`) | ✅ Done |
| CI/CD + automated GitHub Releases | ✅ Done |
| Shell history import (Bash, Zsh, Fish) | ✅ Done |
| Enhanced TUI & Ctrl+R experience | ✅ Done |
| Self-update pipeline (`ggnmem self-update`) | ✅ Done |
| One-line installer (`curl | bash`) | ✅ Done |

### Phase 24+: Future Plans

| Feature | Description |
|---------|-------------|
| Windows PowerShell support | Native Windows binary with PowerShell hooks |
| Ghost-text autosuggestions | Inline command suggestions as you type |
| Fish / Nushell integration | Expand shell support beyond Bash and Zsh |
| Team sharing (opt-in) | Share curated command snippets with your team |
| Plugin system | Extensible hooks for custom indexing and retrieval |

See [docs/roadmap.md](docs/roadmap.md) for the full phased execution plan.

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
```

---

<div align="center">

**Built with Rust 🦀 · Runs 100% locally · No cloud, no telemetry, no compromise.**

[⬆ Back to top](#ggnmem)

</div>
