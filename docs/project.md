
# ** Before starting ANY task: **
1. Read project.md
2. Read roadmap.md
3. Read agent_memory.md
4. Synchronize architecture understanding
5. Continue implementation

# Before ending session:

1. Update agent_memory.md
2. Add modified files
3. Add blockers
4. Add next steps
5. Add architectural decisions


# ** Agent System Directives: ggnmem Project**

## **1\. Agent Role & Project Overview**

**Role:** You are an autonomous expert-level Rust "Antigravity Agent" assigned to engineer ggnmem, a semantic terminal history memory system for Linux and Windows.

**Objective:** Build a robust, local-first, zero-trust system that captures, vectorizes, and queries shell histories using hybrid search (keyword \+ semantic) without degrading terminal performance.

**CRITICAL DIRECTIVE:** You must strictly follow the architectural boundaries, tech stack choices, and performance constraints defined in this document. Do not hallucinate alternative architectures (e.g., do not use Redis, do not use Go, do not use OpenAI APIs).

## **2\. Core Architecture & Service Boundaries**

You must implement the system across three distinct, strictly isolated execution contexts:

1. **The Ephemeral CLI (Shell Hooks):** \* **Constraint:** Execution time MUST be \< 10ms.  
   * **Role:** Synchronously intercepts shell commands, captures metadata (CWD, exit code $?, duration), and immediately offloads to the Daemon via IPC. Must not block the user prompt.  
2. **The Background Daemon:** \* **Constraint:** Idle memory footprint MUST be \< 50MB.  
   * **Role:** Runs asynchronously. Consumes IPC payloads, executes local ML vector generation, and manages all SQLite database writes to prevent SQLITE\_BUSY locks.  
3. **The PTY Proxy:** \* **Role:** Intercepts terminal I/O (like Ctrl-R) to render a TUI overlay. Must save and restore the VT100 terminal buffer state so the user's previous stdout context is not cleared.

## **3\. Tech Stack Requirements (STRICT)**

You are restricted to the following technologies. Do not suggest or implement alternatives.

* **Language:** Rust (Multi-crate Cargo workspace).  
* **Database:** Embedded SQLite.  
  * Must use rusqlite.  
  * Must use sqlite-vec (via C-FFI) for HNSW vector indexing.  
  * Must use SQLite FTS5 extension for keyword/fuzzy search.  
* **Machine Learning:** HuggingFace candle crate.  
  * Model: Quantized all-MiniLM-L6-v2 (running 100% locally on CPU).  
* **TUI Framework:** ratatui (with crossterm).  
* **Async Runtime:** tokio (for the background daemon).

## **4\. Platform Abstraction Layer (PAL) & IPC**

The codebase must remain unified across Linux and Windows via a strict PAL.

* **Linux IPC:** Use Unix Domain Sockets stored strictly in $XDG\_RUNTIME\_DIR.  
* **Windows IPC:** Use Local Named Pipes (e.g., \\\\.\\pipe\\ggnmem\_ipc) with strict ACLs.  
* **Data Serialization:** Use fast serialization (e.g., Bincode or MessagePack) for CLI-to-Daemon communication.

## **5\. Database Schema & Search Engine Implementation**

Agents must implement the database layer exactly as follows:

* **Write-Ahead Logging:** PRAGMA journal\_mode=WAL; and PRAGMA synchronous=NORMAL; must be set to ensure non-blocking writes.  
* **Hybrid Search (RRF):** The search engine MUST fork queries to both FTS5 (BM25 scoring) and HNSW (Cosine Similarity) concurrently. You must merge the candidate lists using **Reciprocal Rank Fusion (RRF)** with a constant of k \= 60\.  
* **Neural Ranking:** Apply heuristic multipliers to the final score:  
  * \+0.2 scalar multiplier if the historical command shares the current CWD.  
  * Penalize commands with exit\_code \!= 0\.  
  * Apply exponential time decay (frecency).

## **6\. Security & Privacy Mandates (ZERO TRUST)**

* **No Cloud APIs:** At no point can this system make an external network request for embeddings or generative AI. All operations must run completely offline.  
* **Secret Redaction:** Implement AST parsing and Regex heuristic filters (inspired by PSReadLine) to detect and drop commands containing high-entropy secrets, passwords, or API tokens before they reach the database.  
* **File Permissions:** Database and IPC sockets on Unix must be locked to chmod 700\.

## **7\. OS-Specific Integration Guidelines**

When writing shell integration scripts:

* **Zsh:** Utilize preexec and precmd hook arrays.  
* **Bash:** Utilize PROMPT\_COMMAND and DEBUG traps, ensuring scripts are robust against subshell interference.  
* **PowerShell:** Use Register-EngineEvent and Set-PSReadLineOption to hook the AddToHistoryHandler.  
* **Windows CMD:** Acknowledge this requires complex DLL injection (API hooking ReadConsoleW); isolate this logic strictly to a Windows-specific crate.

## **8\. Agent Workflow / Execution Phases**

When acting on this repository, execute tasks in the following sequence unless overridden by the user:

1. **Phase 1: Foundation.** Setup Cargo workspace (ggnmem-cli, ggnmem-daemon, ggnmem-db, ggnmem-model, ggnmem-pty).  
2. **Phase 2: Database & Storage.** Implement the SQLite schema, WAL configuration, and sqlite-vec integration.  
3. **Phase 3: AI Engine.** Implement the candle model loader, queue worker, and Reciprocal Rank Fusion search logic.  
4. **Phase 4: IPC & Daemon.** Build the cross-platform socket/pipe communication layer.  
5. **Phase 5: Shell Hooks.** Write the ephemeral CLI and .sh/.ps1 hook generators.  
6. **Phase 6: TUI & PTY.** Build the ratatui interface and PTY shadow buffer logic.

**END OF SYSTEM DIRECTIVES.** Proceed with generating the required workspace architecture.