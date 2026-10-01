# Starter Issues for ggnmem

These issues are designed for new contributors, including students participating in open-source programs (e.g., NSoC 2026). Each issue includes full context, suggested approach, and acceptance criteria.

Create these as GitHub Issues with the appropriate labels.

---

## Issue 1 — Improve Doc Comments on Core Database Module

**Labels:** `good first issue`, `documentation`, `help wanted`
**Difficulty:** 🟢 Beginner

### Context

The `ggnmem-db` crate is the storage foundation of ggnmem. It contains the SQLite database layer, FTS5 full-text search, and the domain model. While the code is well-structured, many public functions and types in `ggnmem-db/src/storage.rs` (74KB, the largest file in the project) lack comprehensive doc comments.

Good documentation helps new contributors understand the codebase and improves IDE support through hover documentation.

### Why It Matters

- New contributors need to understand the database layer to work on search, indexing, or cleanup features
- Doc comments appear in `cargo doc` and IDE tooltips
- Well-documented code is easier to review and maintain

### Suggested Files

- `ggnmem-db/src/storage.rs` — Core `Database` struct and its methods
- `ggnmem-db/src/domain.rs` — Domain types (`CommandRecord`, `SearchQuery`, `SearchResult`, etc.)
- `ggnmem-db/src/filter.rs` — Command ingestion filtering and secret redaction

### Expected Outcome

- All public functions in the target files have `///` doc comments
- Doc comments explain purpose, parameters, return values, and error conditions
- Run `cargo doc -p ggnmem-db --no-deps --open` to verify documentation renders correctly

### Acceptance Criteria

- [ ] All public functions in `storage.rs` have doc comments
- [ ] All public types in `domain.rs` have doc comments
- [ ] `cargo doc -p ggnmem-db --no-deps` compiles without warnings
- [ ] `cargo test --workspace` passes
- [ ] `cargo fmt --all -- --check` passes

---

## Issue 2 — Improve Error Handling in install-online.sh

**Labels:** `good first issue`, `enhancement`, `shell`
**Difficulty:** 🟡 Easy

### Context

The `scripts/install-online.sh` bootstrap installer is the primary way users install ggnmem on Linux and WSL. While it works for the happy path, several edge cases could be handled more gracefully.

### Current Behavior

The script:
1. Detects platform and architecture
2. Queries GitHub API for releases
3. Downloads and extracts the release tarball
4. Installs binaries and configures shell hooks

### Problem

The script could fail without clear guidance in these situations:
- `curl` is not installed
- GitHub API returns a rate-limit error (no authentication)
- Download is interrupted or the tarball is corrupted
- `~/.local/bin` does not exist and cannot be created
- The user does not have write permissions to the install directory
- `tar` extraction fails
- Required shell config files (`.bashrc`, `.zshrc`) don't exist
- The script is run on macOS (currently just prints "Unsupported OS" with no guidance)

### Suggested Approach

1. Add a dependency check at the top (`curl`, `tar`, `grep`, `sed`)
2. Add a `cleanup()` function with a `trap` to remove partial downloads on failure
3. Improve the rate-limit error message to suggest using `GITHUB_TOKEN`
4. Add checksum verification before extraction (if checksums.txt is available)
5. Add a `--dry-run` flag that shows what would happen without making changes
6. Print a helpful message for unsupported platforms (macOS → suggest building from source)

### Expected Outcome

The installer fails gracefully with clear, actionable error messages instead of cryptic shell errors.

### Acceptance Criteria

- [ ] Script checks for required commands before proceeding
- [ ] Partial downloads are cleaned up on failure
- [ ] Rate-limit errors show a helpful message
- [ ] Unsupported platforms get guidance (not just an error)
- [ ] Script passes `shellcheck scripts/install-online.sh` (or at least reduces warnings)
- [ ] All existing functionality continues to work

---

## Issue 3 — Add Help/Keyboard Shortcut Overlay to TUI

**Labels:** `enhancement`, `tui`, `help wanted`
**Difficulty:** 🟡 Medium

### Context

The ggnmem TUI (`ggnmem-cli/src/tui.rs`) is a full-screen ratatui interface with many keyboard shortcuts:

- `Ctrl+F` — Cycle search modes (FTS → Semantic → Hybrid)
- `Tab` — Toggle detail preview panel
- `Enter` — Insert command into shell
- `Shift+Enter` — Execute command immediately
- `Ctrl+C` — Copy selected / quit
- `Ctrl+L` — Clear query
- `Esc` — Exit
- `PgUp/PgDn` — Page navigation
- `Shift+I` — Toggle internal command visibility
- `Shift+P` — Pin command
- `Shift+F` — Favorites only

However, there is no in-TUI help screen. New users must read the source code to discover these shortcuts.

### Expected Behavior

Pressing `?` or `F1` should toggle a centered overlay panel showing all available keyboard shortcuts, grouped by category. Pressing the same key (or `Esc`) should dismiss the overlay.

### Suggested Approach

1. Add a `show_help: bool` field to the TUI state struct
2. Toggle it on `?` or `F1` key press
3. Render a centered `Paragraph` widget with a `Block` border when active
4. Group shortcuts by category (Navigation, Search, Actions, etc.)
5. Use the existing color palette constants for styling

### Acceptance Criteria

- [ ] `?` or `F1` toggles a help overlay
- [ ] The overlay displays all current keyboard shortcuts
- [ ] `Esc` or the same key dismisses the overlay
- [ ] The overlay does not interfere with normal TUI operation
- [ ] The help text is complete and accurate
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes

---

## Issue 4 — Define Plugin System Interface and Module Boundary

**Labels:** `enhancement`, `architecture`, `roadmap`
**Difficulty:** 🔴 Hard

### Context

ggnmem's roadmap mentions "Enhanced custom indexing plugins" (see `docs/roadmap.md`). Currently, the knowledge base (`ggnmem-knowledge`) supports user-defined knowledge packs loaded from `~/.config/ggnmem/knowledge/`, but there is no general plugin architecture for extending ggnmem's core functionality.

A plugin system would allow contributors to:
- Add new shell integrations (Fish, Nushell, PowerShell)
- Add custom command filters or transformers
- Add alternative embedding providers
- Add export formats

### Important: This is NOT a Full Implementation

This issue is about **defining the interface**, not building a runtime plugin loader. The goal is to establish safe architectural boundaries so future contributors can implement plugins against a stable interface.

### Scope

1. Create a new `ggnmem-plugin` crate in the workspace
2. Define a `Plugin` trait with lifecycle methods:
   ```rust
   pub trait Plugin: Send + Sync {
       fn name(&self) -> &str;
       fn version(&self) -> &str;
       fn on_command_captured(&self, command: &str, metadata: &CommandMetadata) -> PluginResult<()>;
       fn on_search_query(&self, query: &str) -> PluginResult<Option<String>>;
   }
   ```
3. Define `PluginResult<T>` error type
4. Define `PluginManifest` struct for plugin metadata
5. Add configuration types for enabling/disabling plugins
6. Write doc comments explaining the plugin contract
7. Add unit tests for the trait interface
8. Document the extension points in a `ggnmem-plugin/README.md`

### Non-Goals

- Dynamic loading (`.so` / `.dll`)
- Plugin distribution or registry
- Network-based plugins
- Breaking changes to existing crates

### Acceptance Criteria

- [ ] `ggnmem-plugin` crate exists with trait definitions
- [ ] Error types are defined
- [ ] At least one example/test plugin implementing the trait
- [ ] Doc comments on all public items
- [ ] `cargo check --workspace` passes
- [ ] `cargo test --workspace` passes
- [ ] Architecture documented in `ggnmem-plugin/README.md`

---

## Issue 5 — Add Comprehensive Tests for Database Search and FTS5

**Labels:** `testing`, `help wanted`, `good first issue`
**Difficulty:** 🟡 Medium

### Context

The `ggnmem-db` crate contains the core search functionality, including FTS5 full-text search with trigram tokenization. While there are tests for migrations and filtering, the **search path** (`storage.rs` search methods, `fuzzy.rs` fuzzy matching) has limited test coverage.

### Testing Gap

After inspection, the following areas need coverage:

1. **FTS5 search** — keyword matching, prefix matching, phrase matching
2. **Fuzzy search** — `fuzzy.rs` contains fuzzy scoring logic
3. **Edge cases** — empty queries, single character, special characters (`*`, `"`, `'`, `\`)
4. **Unicode** — commands with non-ASCII characters
5. **Duplicate handling** — same command from different sessions
6. **CWD boosting** — search results boosted by matching working directory
7. **Run count** — frequency-based ranking

### Suggested Approach

1. Create a test helper function that sets up an in-memory SQLite database with test data
2. Insert a known set of test commands covering various patterns
3. Write test cases for each category above
4. Use `tempfile` for temporary database files (already in dev-dependencies)

### Example Test Structure

```rust
#[cfg(test)]
mod search_tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn setup_test_db() -> Database {
        // Create temp DB, apply migrations, insert test data
    }

    #[test]
    fn search_exact_keyword() { ... }

    #[test]
    fn search_empty_query_returns_recent() { ... }

    #[test]
    fn search_special_characters_do_not_crash() { ... }

    #[test]
    fn search_cwd_boost_ranks_higher() { ... }
}
```

### Acceptance Criteria

- [ ] At least 10 new test cases covering the categories above
- [ ] Tests use temporary databases (no shared state)
- [ ] Tests cover both normal and edge cases
- [ ] All tests pass on Linux and Windows
- [ ] `cargo test -p ggnmem-db` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes
