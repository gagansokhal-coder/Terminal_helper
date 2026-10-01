# Contributing to ggnmem

Welcome, and thank you for your interest in contributing to **ggnmem** — the Semantic Terminal Memory Engine!

Whether you are a first-time contributor, a student participating in an open-source program, or an experienced Rust developer — you are welcome here. Every contribution matters, from fixing a typo to implementing a new feature.

You do **not** need to understand the entire codebase to contribute. Start with what interests you, and ask questions freely.

---

## Table of Contents

- [Code of Conduct and Contributor Safety](#code-of-conduct-and-contributor-safety)
- [Getting Started](#getting-started)
- [Development Environment](#development-environment)
- [Running ggnmem](#running-ggnmem)
- [Testing](#testing)
- [Code Style](#code-style)
- [Security](#security)
- [Pull Requests](#pull-requests)
- [Good First Contributions](#good-first-contributions)

---

## Code of Conduct and Contributor Safety

This project is committed to being a **safe, inclusive, and harassment-free environment** for everyone — regardless of age, experience level, background, or identity.

We have **zero tolerance** for:

- Harassment, intimidation, or threats of any kind
- Offensive, discriminatory, or derogatory comments
- Predatory behavior toward any contributor, especially younger or student contributors
- Inappropriate private contact with contributors
- Requests for personal or private information
- Sexual or romantic solicitation
- Doxxing or sharing private information without consent

All contributors must read and follow our [Code of Conduct](CODE_OF_CONDUCT.md).

### Reporting Safety Concerns

If you experience or witness any behavior that violates this policy, please report it **privately** to the project administrator:

📧 **gaganrajput1008@gmail.com**

All reports will be handled confidentially.

---

## Getting Started

### Workflow Overview

```
Fork the repository on GitHub
    ↓
Clone your fork locally
    ↓
Install Rust toolchain
    ↓
Build the workspace
    ↓
Run tests
    ↓
Start the daemon (optional)
    ↓
Launch the TUI (optional)
    ↓
Make your changes
    ↓
Run all checks
    ↓
Create a feature branch
    ↓
Commit with a descriptive message
    ↓
Push and open a Pull Request
```

### Step-by-Step

1. **Fork** this repository on GitHub.

2. **Clone** your fork:
   ```bash
   git clone https://github.com/YOUR_USERNAME/Terminal_helper.git
   cd Terminal_helper
   ```

3. **Build** the workspace:
   ```bash
   cargo build --workspace
   ```

4. **Run tests**:
   ```bash
   cargo test --workspace
   ```

5. **Run quality checks**:
   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   ```

6. **Create a branch** for your work:
   ```bash
   git checkout -b feat/my-improvement
   ```

7. **Commit** your changes:
   ```bash
   git add .
   git commit -m "feat: add description of change"
   ```

8. **Push** and open a Pull Request:
   ```bash
   git push origin feat/my-improvement
   ```

---

## Development Environment

### Requirements

| Requirement | Version | Notes |
|------------|---------|-------|
| **Rust** | 1.76+ (stable) | MSRV specified in `Cargo.toml` |
| **Cargo** | Comes with Rust | |
| **Git** | Any recent version | |

### Platform-Specific Notes

**Linux / WSL:**
- No additional system dependencies required (SQLite is bundled via `rusqlite`)
- Bash or Zsh for shell hook testing

**Windows:**
- MSVC toolchain recommended (Visual Studio Build Tools)
- The `.cargo/config.toml` configures dynamic CRT (`/MD`) for ONNX Runtime compatibility
- PowerShell for testing the Windows installer

### Optional: Dev Container

A `.devcontainer` configuration is provided for VS Code / GitHub Codespaces:
```bash
# Opens in a pre-configured container with Rust toolchain
code --devcontainer .
```

---

## Running ggnmem

After building (`cargo build --workspace`), the binaries are in `target/debug/`:

### Initialize Shell Integration

```bash
# Generate and display the shell hook (do NOT pipe into eval during development)
cargo run --bin ggnmem-cli -- init bash
cargo run --bin ggnmem-cli -- init zsh
```

### Start the Daemon

```bash
# Start the background daemon
cargo run --bin ggnmem-daemon
```

Or via the CLI:
```bash
cargo run --bin ggnmem-cli -- start
```

### Check Status

```bash
cargo run --bin ggnmem-cli -- status
cargo run --bin ggnmem-cli -- doctor
```

### Launch the TUI

```bash
cargo run --bin ggnmem-cli -- ui
```

### Search

```bash
cargo run --bin ggnmem-cli -- search "git commit"
cargo run --bin ggnmem-cli -- semantic "show running containers"
```

### Import History

```bash
cargo run --bin ggnmem-cli -- import auto
cargo run --bin ggnmem-cli -- import bash --dry-run
```

---

## Testing

### Run All Tests

```bash
cargo test --workspace
```

### Run Tests for a Specific Crate

```bash
cargo test -p ggnmem-db
cargo test -p ggnmem-ai
cargo test -p ggnmem-daemon
```

### Run a Specific Test

```bash
cargo test -p ggnmem-db -- migrations_are_idempotent
```

### Formatting Check

```bash
cargo fmt --all -- --check
```

### Lint Check (Clippy)

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

### Full Pre-PR Check

Run all checks before opening a PR:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

---

## Code Style

### Formatting

We use `rustfmt` with the configuration in `rustfmt.toml`:
- Edition 2021
- Max line width: 100
- Unix newlines
- Field init shorthand enabled
- Try shorthand (`?`) enabled

Run `cargo fmt --all` to auto-format before committing.

### Naming Conventions

- **Types**: `PascalCase` (e.g., `CommandRecord`, `DaemonConfig`)
- **Functions/methods**: `snake_case` (e.g., `apply_migrations`, `search_commands`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `PROTOCOL_VERSION`, `FTS_WEIGHT`)
- **Crate names**: `ggnmem-*` prefix for all workspace crates

### Error Handling

- Use `anyhow::Result` in binaries and CLI code
- Use `thiserror` for library error types (see `ggnmem-db/src/error.rs`)
- Avoid `.unwrap()` in library code; use `?` propagation
- Document error conditions in doc comments

### Logging

- Use `tracing` for structured logging in the daemon
- Use `println!` / `eprintln!` only in CLI user-facing output

### Documentation

- Add doc comments (`///`) to all public types and functions
- Include usage examples where helpful
- Module-level documentation (`//!`) for each module

### Dependencies

- **Do not** add new dependencies without discussion
- Prefer workspace-level dependency declarations
- The project deliberately avoids large framework dependencies
- `unsafe` code is **forbidden** workspace-wide (see `Cargo.toml` lint config)

---

## Security

This project handles sensitive data (terminal command history). We take security seriously.

### Prohibited in Contributions

**Never** include in any commit:
- API keys, passwords, authentication tokens, or private keys
- Real user command history or terminal output
- Personal data (names, emails, IP addresses)
- Machine-specific configuration or paths
- `.env` files or environment variable dumps

Use placeholders instead:
```
YOUR_API_KEY
/path/to/example
user@example.com
```

### Automated Checks

Our CI pipeline and review process may reject PRs that contain patterns resembling sensitive data. The `ggnmem-db` crate already includes a **secret redaction filter** (`filter.rs`) that prevents ingestion of commands containing credentials.

See [SECURITY.md](SECURITY.md) for our vulnerability disclosure policy.

---

## Pull Requests

### Branch Naming

Use descriptive branch names with a prefix:

```
feat/add-fish-hook           # New features
fix/daemon-socket-cleanup    # Bug fixes
docs/improve-architecture    # Documentation
test/db-search-coverage      # Tests
refactor/split-tui-module    # Refactoring
chore/update-dependencies    # Maintenance
```

### PR Guidelines

- **Keep PRs small and focused.** One logical change per PR.
- **Write descriptive commit messages.** Use [Conventional Commits](https://www.conventionalcommits.org/) style:
  ```
  feat: add Fish shell hook integration
  fix: prevent daemon crash on empty search query
  docs: add architecture diagram for IPC flow
  test: add coverage for FTS5 special character handling
  ```
- **Include tests** for new functionality.
- **Update documentation** if behavior changes.
- **Include screenshots** for TUI/UI changes (before + after).
- **Pass all CI checks** before requesting review.

### Review Process

- A maintainer will review your PR within a reasonable timeframe.
- Be responsive to feedback — we aim for constructive, respectful reviews.
- Don't be discouraged by change requests; they are part of the process.

---

## Good First Contributions

Looking for a place to start? Here are some beginner-friendly areas:

- 📝 **Documentation** — Improve doc comments on public functions, add examples
- 🧪 **Tests** — Add unit tests for untested modules (especially `ggnmem-db/src/storage.rs`)
- 💬 **Error messages** — Make CLI error messages more user-friendly
- 📖 **Examples** — Create usage examples or tutorials
- 🖥️ **TUI improvements** — Color tweaks, layout improvements, help text
- 🐚 **Shell compatibility** — Improve hooks for edge cases (Fish, Nushell)
- 🐛 **Bug fixes** — Check the issue tracker for `good first issue` labels

Check the [Issues](https://github.com/gagansokhal-coder/Terminal_helper/issues) page for issues labeled `good first issue` or `help wanted`.

---

## Questions?

If you have questions about the codebase, architecture, or how to approach a contribution:

1. Open a [Discussion](https://github.com/gagansokhal-coder/Terminal_helper/discussions) on GitHub
2. Comment on the relevant Issue
3. Ask in your Pull Request

We are happy to help newcomers. No question is too basic.
