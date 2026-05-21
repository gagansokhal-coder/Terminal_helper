# **Antigravity Agent Skill Requirements: ggnmem Project**

To successfully engineer the ggnmem semantic terminal history system, the assigned Antigravity Agent (e.g., Gemini 3.1 Pro, Claude 3.5 Sonnet) must possess and actively utilize the following deep technical competencies.

## **1\. Primary Language Proficiency: Systems Rust**

The agent must write idiomatic, high-performance, and safe Rust code, specifically targeted at systems-level programming.

* **Zero-Cost Abstractions:** Deep understanding of memory management without garbage collection (ownership, borrowing, lifetimes).  
* **Concurrency (tokio):** Expertise in asynchronous programming, building non-blocking event loops, threading, and utilizing worker pools for heavy CPU tasks (like ML tensor generation).  
* **C-FFI (Foreign Function Interface):** Ability to safely bind and interact with C libraries, specifically for linking SQLite extensions.  
* **Workspace Management:** Proficiency in structuring multi-crate Cargo workspaces to ensure decoupled domain logic.

## **2\. Database & Data Engineering**

The agent must act as an expert DBA for embedded systems.

* **Advanced SQLite:** Beyond basic CRUD. Must understand SQLite internals, specifically:  
  * Write-Ahead Logging (WAL) configuration and synchronous modes for concurrency.  
  * Memory-mapped I/O (mmap\_size).  
  * Virtual tables.  
* **Full-Text Search (FTS5):** Expertise in configuring FTS5, specifically utilizing trigram tokenizers for fuzzy matching and BM25 ranking algorithms.  
* **Vector Databases (sqlite-vec):** Deep knowledge of integrating and querying vector embeddings within SQLite using C extensions. Understanding of Hierarchical Navigable Small World (HNSW) graphs, Cosine Similarity, and Approximate Nearest Neighbor (ANN) search.

## **3\. Machine Learning & Local AI Engineering**

The agent must implement AI features without relying on cloud APIs.

* **Local Tensor Operations (candle):** Expertise in using HuggingFace's Rust-native candle crate for loading, running, and managing quantized machine learning models entirely on the CPU.  
* **Embedding Generation:** Understanding of how to tokenize text and generate 384-dimensional dense vector representations (specifically using models like all-MiniLM-L6-v2).  
* **Information Retrieval Algorithms:** Proficiency in implementing mathematical algorithms like Reciprocal Rank Fusion (RRF) to merge unbounded BM25 scores with bounded semantic vector scores.

## **4\. Operating System Internals & IPC**

The agent must build low-level, cross-platform bridges.

* **Inter-Process Communication (IPC):**  
  * *Linux:* Unix Domain Sockets and socket activation.  
  * *Windows:* Local Named Pipes, Access Control Lists (ACLs), and the Windows API (windows-rs).  
* **Filesystem Standards:** Adherence to XDG Base Directory Specifications on Linux and appropriate AppData usage on Windows.  
* **Daemon/Service Architecture:** Understanding of background process lifecycles, daemonization, and systemd user services.

## **5\. Shell Scripting & Terminal Emulation**

The agent must manipulate the volatile environment of terminal emulators and user shells.

* **Shell Hooks Internals:** Intimate knowledge of the execution pipelines for:  
  * *Zsh:* preexec and precmd arrays.  
  * *Bash:* PROMPT\_COMMAND and DEBUG traps, handling subshell complexities.  
  * *PowerShell:* Register-EngineEvent, AST parsing, and the PSReadLine module.  
* **Pseudoterminals (PTY):** Deep understanding of VT100 escape sequences, raw mode terminal interaction, and how to create proxy layers that intercept and replay standard input/output without corrupting the shadow buffer.

## **6\. Terminal User Interface (TUI) Development**

* **Immediate-Mode Rendering (ratatui):** Expertise in building non-blocking, responsive terminal interfaces using Rust's ratatui ecosystem.  
* **Event Handling (crossterm):** Managing cross-platform keyboard events, mouse tracking, and terminal resizing gracefully.

## **7\. Security & Cryptography**

* **Zero-Trust Architecture:** Implementing local-first designs where sensitive data never leaves the machine.  
* **Data Sanitization:** Utilizing Abstract Syntax Tree (AST) parsing and complex Regular Expressions to build redaction layers that filter out secrets (passwords, API keys) before ingestion.  
* **Encryption at Rest (Optional Phase):** Knowledge of integrating SQLCipher or similar AES-256-GCM database encryption and interfacing with OS native credential managers (Linux Secret Service, Windows Credential Manager).