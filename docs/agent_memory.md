# ggnmem — Persistent Agent Memory

## Purpose

This file acts as the persistent cross-agent memory layer for the ggnmem project.

All AI agents (Antigravity, Codex, Kilo/Keo, VSCode agents, Gemini, Claude, etc.) MUST:

1. Read this file BEFORE performing any task.
2. Synchronize understanding with:

   * project.md
   * architecture.md
   * roadmap.md
   * skills.md
3. Continue implementation strictly according to the existing architecture.
4. Append updates to this file BEFORE ending the session.

This file is the single source of truth for:

* current implementation state
* completed work
* pending work
* architectural decisions
* blockers
* debugging notes
* modified files
* future direction

---

# Project Overview

Project Name:
ggnmem

Project Type:
Cross-platform semantic terminal memory engine.

Primary Goal:
Build a local-first, privacy-focused terminal history intelligence system for Linux and Windows.

Core Features:

* terminal history indexing
* keyword search
* fuzzy search
* semantic vector search
* contextual ranking
* shell integration
* daemon architecture
* offline embeddings
* TUI interface
* PTY overlay rendering

Primary Design Philosophy:

* local-first
* zero-trust
* offline AI
* low latency
* modular architecture
* UNIX-style tooling
* cross-platform abstractions

---

# Source of Truth Hierarchy

Priority order:

1. project.md
2. architecture.md
3. roadmap.md
4. agent_memory.md
5. skills.md

If conflicts occur:
Higher-priority files override lower-priority files.

---

# Mandatory Agent Workflow

Before starting ANY task:

1. Read:

   * project.md
   * architecture.md
   * roadmap.md
   * agent_memory.md
   * skills.md

2. Analyze:

   * current implementation state
   * completed phases
   * existing architecture
   * unresolved blockers

3. Continue implementation WITHOUT:

   * rewriting architecture
   * changing tech stack
   * introducing cloud APIs
   * replacing SQLite
   * replacing Rust
   * changing daemon boundaries

4. Maintain:

   * strict modularity
   * cross-platform abstraction
   * zero-trust principles
   * low-memory constraints

---

# Hard Constraints

Agents MUST NOT:

* use external AI APIs
* upload user data
* replace SQLite
* replace Rust
* introduce Redis/Postgres
* introduce Electron
* break local-first architecture
* block shell prompt
* exceed CLI execution latency constraints
* create monolithic architecture
* bypass IPC boundaries

---

# Current Project Status

Current Phase:
Phase 1 — Foundation & Workspace Initialization

Overall Progress:
0%

Current Objective:
Initialize production-grade Rust Cargo workspace and repository structure.

Target Workspace Crates:

* ggnmem-cli
* ggnmem-daemon
* ggnmem-db
* ggnmem-model
* ggnmem-pty

Primary Current Goal:
Implement foundational project scaffolding before database/search logic.

---

# Current Known Architecture

Core Runtime Layers:

1. Ephemeral CLI Layer

* shell hooks
* synchronous execution
* IPC payload forwarding

2. Background Daemon

* async runtime
* SQLite writes
* embedding generation
* indexing

3. PTY Proxy

* VT100 interception
* overlay rendering
* shadow buffer restoration

Core Search Pipeline:

* FTS5 keyword search
* trigram fuzzy matching
* sqlite-vec semantic search
* Reciprocal Rank Fusion (RRF)

Primary Database:
SQLite

Primary Runtime:
Tokio

Primary TUI:
ratatui + crossterm

Primary ML Runtime:
candle

---

# Current Tasks Queue

## Highest Priority

* initialize Cargo workspace
* create crate structure
* setup workspace dependencies
* create architecture-safe module boundaries

## Secondary Priority

* SQLite schema
* WAL configuration
* IPC layer
* shell history parsing

## Deferred

* semantic embeddings
* PTY overlay
* AI workflows
* enterprise synchronization

---

# Pending Engineering Decisions

Unresolved:

* exact IPC serialization format
* config file schema
* plugin loading architecture
* shell initialization UX
* migration/versioning strategy

---

# Performance Constraints

CLI Layer:
< 10ms execution time

Daemon:
< 50MB idle memory

Search:
Sub-100ms perceived latency

Database:
Must use WAL mode

Embedding Pipeline:
Must remain asynchronous

---

# Security Constraints

Mandatory:

* local-only execution
* no telemetry
* no cloud sync
* no external embeddings APIs
* permissions lockdown
* secret redaction layer

Potential Future:

* SQLCipher encryption
* OS credential manager integration

---

# Agent Session Log Format

Every agent MUST append sessions using this exact format.

---

## Session Log Template

### Session

Date:
Agent:
Model:

### Completed

* item

### Modified Files

* path/file

### Architectural Decisions

* decision

### Problems Encountered

* issue

### Current State

* explanation

### Next Recommended Steps

* next tasks

### Warnings

* important notes

---

# Current Session State

No implementation completed yet.

Repository currently exists only at planning/documentation stage.

Next immediate step:
Initialize Rust Cargo workspace and repository structure.

---

# Final Directive

Agents must optimize for:

* maintainability
* performance
* modularity
* trust
* reproducibility
* low-level engineering quality

This project is intended to evolve into a serious open-source infrastructure tool, not a tutorial/demo repository.
