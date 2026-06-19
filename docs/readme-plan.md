# README Structure

## 1. Hero Section
* **Content:** Project name (`ggnmem`), one-line description ("Semantic Terminal Memory Engine"), and a placeholder for badges (CI status, latest release, license, platform support).

## 2. Why ggnmem?
* **Content:** Problem statement regarding traditional shell history (flat log, keystrokes instead of intent). Explain why developers forget commands (complex flags, infrequent use). Detail how ggnmem solves this by capturing and understanding intent, saving time and mental effort.

## 3. Features
* **Content:** High-level list of key capabilities:
  * Automatic command capture (< 10ms latency)
  * Hybrid search (FTS5 keyword + semantic intent)
  * AI-powered local embeddings (MiniLM, BGE)
  * Interactive full-screen TUI
  * Seamless shell history import (Bash/Zsh/Fish)
  * Automated self-update mechanism
  * One-line installation script

## 4. Quick Install
* **Content:** The fastest way to get started. Include the `curl | bash` command and specify supported platforms (Linux x86_64/aarch64, WSL).

## 5. Quick Start
* **Content:** A brief, step-by-step guide from starting the daemon to running the first search. Include commands like `ggnmem doctor`, `ggnmem start`, and `ggnmem ui`.

## 6. Example Usage
* **Content:** 5-10 realistic examples showing how users can search for commands (e.g., "find large files", "compress a directory"). Show how semantic search retrieves the correct underlying command even if the exact words aren't used.

## 7. Architecture
* **Content:** High-level architectural overview. Use a Markdown diagram illustrating the flow: Shell hook -> Daemon (Ingestion/Embedding/Search) -> SQLite (FTS5 + Vector Store) -> Output. Briefly outline the crate structure.

## 8. AI Features
* **Content:** Explain the local-first semantic search capabilities. Detail the embedding process using lightweight ONNX models, emphasizing that no cloud dependency or internet connection is required after setup.

## 9. Installation Guide
* **Content:** Detailed installation steps. Include manual installation methods, building from source, and a link to the comprehensive `INSTALL.md` for advanced configurations.

## 10. Updating
* **Content:** Instructions on how to keep the software current using `ggnmem self-update` and alternative manual upgrade paths.

## 11. Privacy
* **Content:** Reiterate the local-first philosophy. Highlight zero network requests, secret redaction, and strict local data storage.

## 12. Roadmap
* **Content:** A snapshot of future plans (e.g., Phase 26+ goals). Mention upcoming features like Windows PowerShell support and ghost-text autosuggestions. Provide a link to the full roadmap document.

## 13. Contributing
* **Content:** Guidelines for developers who want to contribute. Mention formatting (`cargo fmt`), linting (`cargo clippy`), and testing requirements.

## 14. License
* **Content:** Mention the current license (MIT) and a brief copyright notice. Include a link to the full `LICENSE` file.
