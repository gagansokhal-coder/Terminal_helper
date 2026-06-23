# ggnmem Roadmap

## Current Architecture Summary

`ggnmem` is a local-first semantic terminal memory engine built around three isolated runtime contexts:

1. **Ephemeral CLI and shell hooks**
   - Captures command metadata from the active shell.
   - Must return in under 10ms.
   - Sends serialized payloads to the daemon over IPC.
   - Must never run embeddings, search, or database-heavy work inline.

2. **Background daemon**
   - Owns SQLite writes, embedding generation, indexing, and search.
   - Uses Tokio for async orchestration.
   - Keeps idle memory below 50MB.
   - Prevents multi-process SQLite lock contention by centralizing writes.

3. **PTY proxy**
   - Intercepts terminal input/output for overlay search UX.
   - Maintains VT100 shadow state.
   - Restores terminal output context after interaction.

The architecture is intentionally modular and must remain a Rust multi-crate Cargo workspace with these crate boundaries:

- `ggnmem-cli`
- `ggnmem-daemon`
- `ggnmem-db`
- `ggnmem-model`
- `ggnmem-pty`
- `ggnmem-ai`
- `ggnmem-knowledge`
- `ggnmem-paths`

The approved stack remains:

- Rust
- Tokio
- SQLite through `rusqlite`
- SQLite WAL and FTS5
- `sqlite-vec` through C-FFI
- ONNX Runtime (`ort` crate)
- Quantized `all-MiniLM-L6-v2`
- `ratatui` with `crossterm`
- Unix Domain Sockets on Linux
- Local Named Pipes on Windows

No cloud APIs, external embedding services, Redis, Postgres, Go rewrite, Electron UI, or monolithic process design belong in the roadmap.

## Current Project Maturity

Current maturity: **active pre-alpha stage**.

The project has a robust architecture and the core engine is fully implemented. The Cargo workspace contains the required crates, Linux and Windows binaries are built in CI, and full FTS/semantic search works locally.

Current status:

- Architecture is defined and implemented.
- Technology choices are fixed and validated.
- Core crates are created and actively maintained.
- Local command capture, storage, semantic indexing, and search work.
- TUI overlay functions as a `Ctrl+R` replacement.
- Windows native installation and build pipelines are complete.
- Distribution pathways (install scripts, self-update) are operational.

## MVP Boundaries

The MVP should prove the core local semantic history loop on Linux before adding advanced UI and Windows complexity.

MVP includes:

- Linux-only first release.
- Rust Cargo workspace with the five required crates.
- Zsh command capture path.
- Bash command capture path where practical, with documented limitations.
- Ephemeral CLI that serializes command payloads and exits under 10ms.
- Linux daemon listening on a Unix Domain Socket under `$XDG_RUNTIME_DIR`.
- SQLite database stored under the XDG data directory.
- WAL mode and `synchronous=NORMAL`.
- Core command schema with session, command, cwd, exit code, duration, timestamp, and content hash.
- Secret redaction before persistence.
- FTS5 keyword/fuzzy search.
- Local embedding generation through ONNX Runtime (`ort`).
- `sqlite-vec` semantic indexing.
- Hybrid search using RRF with `k = 60`.
- Contextual ranking for CWD match, failed command penalty, and frecency.
- Interactive CLI search UI.

MVP excludes:

- Windows support.
- CMD integration.
- DLL injection.
- Enterprise sync.
- Cloud sync.
- Generative assistant workflows.
- SQLCipher encryption unless required by security review.
- Full PTY overlay polish.
- Ghost-text autosuggestions.
- Cross-machine knowledge graph features.

The first MVP should answer one question well: can `ggnmem` capture Linux shell history locally, protect secrets, index it, and retrieve useful results quickly with hybrid search?

## Phased Execution Plan

### Phase 1 — Foundation

Goal: establish the production workspace without implementing feature logic prematurely.

Deliverables:

- Cargo workspace initialized.
- Required crates created.
- Shared error and configuration boundaries defined inside the approved crates.
- Minimal binaries compile.
- No cross-crate shortcuts that collapse daemon, CLI, database, model, or PTY boundaries.

Milestone:

- `cargo check` succeeds for the workspace.
- Crate ownership is clear.
- No external services or unapproved dependencies are introduced.

### Phase 2 — Database Foundation

Goal: create the local storage contract.

Deliverables:

- SQLite connection setup through `rusqlite`.
- WAL and `synchronous=NORMAL` configuration.
- Initial schema and migrations.
- Command/session persistence.
- Content hash deduplication strategy.
- FTS5 virtual table.
- Early database tests.

Milestone:

- Commands can be inserted and queried locally through the database crate.
- Database initialization is idempotent.
- WAL configuration is verified by tests.

### Phase 3 — Linux IPC and Daemon

Goal: make the CLI-to-daemon path real without semantic indexing yet.

Deliverables:

- Unix Domain Socket IPC under `$XDG_RUNTIME_DIR`.
- Fast serialization format selected from the approved options.
- Daemon listener and payload ingestion.
- Single-writer database path.
- CLI send command with timeout behavior.
- Graceful behavior when daemon is unavailable.

Milestone:

- A Linux shell hook can submit command metadata to the daemon without blocking normal prompt usage.
- CLI latency budget is measured and documented.

### Phase 4 — Shell Capture

Goal: integrate real Linux shells.

Deliverables:

- Zsh `preexec` and `precmd` integration.
- Bash `PROMPT_COMMAND` and `DEBUG` trap integration.
- Hook generator through the CLI.
- Capture of command, cwd, exit code, duration, timestamp, and session identity.
- Redaction before persistence.

Milestone:

- Zsh capture works reliably for normal commands.
- Bash capture works for common cases, with known limitations documented.
- Secrets are dropped before database insertion.

### Phase 5 — Search MVP

Goal: deliver useful local retrieval.

Deliverables:

- FTS5 keyword/fuzzy search with BM25 ranking.
- ONNX model loading.
- Quantized `all-MiniLM-L6-v2` embedding generation.
- `sqlite-vec` integration.
- HNSW vector search.
- RRF merge with `k = 60`.
- CWD, exit-code, and frecency scoring adjustments.
- Interactive CLI search command.

Milestone:

- A user can search historical commands by exact text, fuzzy text, and semantic intent.
- Search quality is acceptable on a realistic local history corpus.
- Search remains local-only.

### Phase 6 — PTY and TUI

Goal: turn search into an interactive terminal experience.

Deliverables:

- PTY proxy foundation.
- VT100 shadow buffer handling.
- Basic `ratatui` search overlay.
- Keyboard navigation.
- Result selection and shell insertion.
- Terminal restore behavior.

Milestone:

- Ctrl-R style interactive search works without destroying terminal context.
- Overlay remains responsive on common terminal sizes.

### Phase 7 — Hardening and Pre-Release

Goal: prepare a Linux pre-alpha release.

Deliverables:

- Performance benchmarks.
- Security review.
- Install and uninstall flow.
- Basic documentation.
- Failure-mode tests.
- Packaging plan for Linux.

Milestone:

- A cautious Linux pre-alpha can be used by early adopters who understand the project is still evolving.

### Phase 18 — Knowledge Base (Completed)

Goal: provide built-in command documentation and learning.

Deliverables:

- `ggnmem-knowledge` crate for offline command packs.
- `ggnmem ask` to query commands naturally.
- `ggnmem explain` to break down command flags.
- `ggnmem learn` for topic-based learning.

Milestone:

- Users can query and learn commands locally without network calls.

### Phase 19 — Release Automation (Completed)

Goal: automate the release pipeline using GitHub Actions.

Deliverables:

- Multi-job CI pipeline (`ci.yml`).
- Release pipeline (`release.yml`) triggered by tags.
- Cross-compilation for `x86_64` and `aarch64`.
- Version verification across Cargo.toml, tag, and binary.
- Auto-generated release notes and checksums.

Milestone:

- Releases are fully automated and verified.

### Phase 20 — Shell History Import (Completed)

Goal: let new users import existing Bash, Zsh, and Fish shell history.

Deliverables:

- `ggnmem import auto` auto-detect and import.
- `ggnmem import bash/zsh/fish` explicit import.
- `--dry-run` and `--preview` safety modes.
- Batch insert with deduplication for 100k+ entries.
- Doctor integration reporting available history files.
- Updated README, INSTALL.md, and roadmap documentation.

Milestone:

- Users can populate ggnmem with their existing shell history in seconds.

### Phase 21 — Enhanced TUI & Ctrl+R Experience (Completed)

Goal: polish the TUI into a powerful command recall interface with refined keybindings and navigation.

Deliverables:

- `Ctrl+F` cycles search modes: FTS → Semantic → Hybrid (replaces Ctrl+S/Ctrl+H).
- `Ctrl+C` Option C behavior: copies selected command (stays in UI), quits when nothing is selected.
- `PgUp/PgDn` for page navigation (10 items per page).
- `Ctrl+Home/Ctrl+End` to jump to first/last result.
- Empty state with helpful tips when search yields no matches.
- Status bar shows database command count alongside results and latency.
- Preview panel displays search source (FTS/SEM/HYB) with color coding.
- `ggnmem doctor` reports TUI availability and clipboard tool detection.
- Updated footer help bar reflecting all new keybindings.

Milestone:

- TUI feels like a modern fuzzy finder with full keyboard navigation and instant feedback.

### Phase 22 — Self-Update Pipeline (Completed)

Goal: Fix large binary download timeouts and standardize the update paths.

Deliverables:

- Granular execution steps: check, download, verify, extract, install.
- Increased connection (30s) and read (120s) timeouts.
- Console progress logging for each update phase.
- Unified download logic for installation and `--download-only` modes.
- Regression tests for update timeouts.

Milestone:

- Self-update reliably downloads and installs large binaries over slow networks without prematurely timing out.

### Phase 23 — Installer & Distribution (Completed)

Goal: provide a frictionless, one-line installation experience for users.

Deliverables:

- `install-online.sh` bootstrap script.
- Automatic platform and architecture detection (Linux x86_64, aarch64).
- GitHub API integration for dynamic release discovery.
- Secure checksum verification (SHA256).
- Safe extraction and artifact validation.
- Rollback-protected execution of the bundled `install.sh`.

Milestone:

- Users can install `ggnmem` securely via a single `curl | bash` command without manual version hunting.

### Phase 24 — Documentation (Completed)

Goal: provide high-quality documentation for users and contributors.

Deliverables:

- Comprehensive README.md with features, architecture, and quickstart.
- INSTALL.md with detailed platform-specific installation steps.
- Visual assets (screenshots/GIFs) showing core features.
- Architecture and design documents updated.

Milestone:

- The project is fully documented and ready for early public adoption.

### Phase 25 — Landing Page (Completed)

Goal: create a modern, SEO-friendly landing page for ggnmem.

Deliverables:

- Next.js + TypeScript static website.
- Dark theme, modern glassmorphism design.
- Sections for Hero, Features, Screenshots, How It Works, Quick Start, Examples, Stats, and Roadmap.
- Full SEO metadata, OpenGraph, sitemap.xml, and robots.txt.
- Vercel deployment configuration.

Milestone:

- The project has a professional online presence at `ggnmem.mytechy.in`.

### Phase 25.1 — Release Workflow Modernization (Completed)

Goal: Update the automated GitHub release pipeline to advertise the new primary install/upgrade workflows.

Deliverables:

- GitHub release template updated to feature the website link.
- One-line curl installer highlighted as the primary installation method.
- `self-update` command highlighted as the primary upgrade method.
- Internal build scripts and generators updated.

Milestone:

### Phase 26A — Windows Release Assets in CI (Completed)

Goal: Build native Windows binaries and package them for releases.

Deliverables:

- `x86_64-pc-windows-msvc` target added to GitHub Actions release pipeline.
- `ggnmem-windows-x86_64.zip` release artifact generated automatically.
- Checksums generation updated for `.zip` format.

Milestone:

- Automated pipelines now output ready-to-run Windows artifacts for every release.

### Phase 26B — Windows PowerShell Installer (Completed)

Goal: Frictionless installation experience on Windows 10/11.

Deliverables:

- `install.ps1` PowerShell bootstrap script.
- Automated fetching and checksum verification for Windows binaries.
- Config, data, and models path correctly routed to `%LOCALAPPDATA%` and `%APPDATA%`.
- `ggnmem self-update` supports Windows in-place upgrades.
- Path modification and profile hook instructions included.

Milestone:

- Windows users can install and manage `ggnmem` securely via PowerShell using `irm | iex`.

## Linux-First Rollout

Linux is the first supported platform because it exercises the core architecture with less platform-specific risk than Windows.

Linux rollout order:

1. Local development build.
2. Zsh-only capture prototype.
3. SQLite persistence and FTS search.
4. Daemon IPC through Unix Domain Sockets.
5. Bash support.
6. Local embeddings and semantic search.
7. PTY/TUI overlay.
8. Installer script.
9. Statically linked release artifact where practical.
10. Homebrew and Nix packaging after the binary interface stabilizes.

Linux support requirements:

- Use XDG directories correctly.
- Place sockets only under `$XDG_RUNTIME_DIR`.
- Lock down sensitive directories and socket paths.
- Support user-level daemon lifecycle.
- Keep the shell hook fast even when daemon or model indexing is unavailable.

## Future Windows Support

Windows support has begun through Phase 26, which established native builds and installation pathways. Further integration requires expanding shell hooks and platform abstractions.

Windows support order:

1. Local Named Pipe IPC with strict ACLs (Implemented).
2. Windows filesystem paths and AppData placement (Implemented).
3. `install.ps1` installer and `self-update` support (Implemented).
4. PowerShell integration through PSReadLine and `Set-PSReadLineOption` (Planned).
5. Windows Terminal VT behavior validation (Planned).
6. MSI or `winget` packaging (Planned).
7. CMD support research (Planned).
8. CMD support implementation only if the DLL injection risk is accepted and isolated to a Windows-specific crate (Planned).

Windows MVP initially targets PowerShell, not CMD.

CMD support remains deferred because reliable capture requires invasive Windows-specific API hooking. It must not contaminate the cross-platform core.

## Testing Stages

### Stage 1 — Compile and Unit Tests

- Workspace `cargo check`.
- Unit tests for serialization, redaction, scoring, and schema initialization.
- Database tests using temporary SQLite files.

### Stage 2 — Integration Tests

- CLI-to-daemon IPC tests.
- Daemon persistence tests.
- FTS5 query tests.
- Embedding queue tests.
- RRF ranking tests with fixed fixtures.

### Stage 3 — Shell Tests

- Zsh hook behavior.
- Bash hook behavior.
- Exit-code capture.
- Duration capture.
- CWD capture.
- Daemon unavailable behavior.

### Stage 4 — Performance Tests

- CLI hook latency under 10ms.
- Daemon idle memory under 50MB.
- Search perceived latency under 100ms for normal datasets.
- Large-history indexing behavior.
- Backpressure behavior during command bursts.

### Stage 5 — Terminal UX Tests

- TUI rendering under common terminal sizes.
- Ctrl-R interception.
- VT100 restore behavior.
- Keyboard navigation.
- Resize handling.

### Stage 6 — Packaging Tests

- Fresh install.
- Upgrade.
- Uninstall.
- Shell init regeneration.
- Permission verification.

## Security Stages

### Stage 1 — Local-Only Guarantee

- Confirm no embedding or search path performs network calls.
- Keep model execution local.
- Avoid telemetry.

### Stage 2 — Filesystem and IPC Permissions

- Linux database directory permissions locked down.
- Linux socket path restricted under `$XDG_RUNTIME_DIR`.
- Windows Named Pipe ACL design reviewed before implementation.

### Stage 3 — Secret Redaction

- Regex detection for obvious secrets.
- AST-aware parsing where available.
- High-entropy token detection.
- Safe-list handling for harmless commands.
- Tests proving rejected commands never reach SQLite or embeddings.

### Stage 4 — Abuse and Failure Cases

- Malformed IPC payloads.
- Oversized commands.
- Binary terminal input.
- Corrupted database startup.
- Interrupted daemon writes.

### Stage 5 — Optional At-Rest Encryption

SQLCipher and OS credential manager integration are future hardening items, not MVP requirements. They should be considered after the core Linux release is stable.

## Release Plan

### 0.1.0-dev

Purpose: internal development baseline.

Criteria:

- Workspace compiles.
- Database crate initializes schema.
- Basic CLI and daemon binaries exist.

### 0.2.0-linux-capture

Purpose: prove Linux command capture.

Criteria:

- Zsh capture works.
- Bash capture works for common cases.
- Daemon receives payloads over Unix Domain Socket.
- SQLite persistence works.
- Redaction layer is active.

### 0.3.0-linux-search

Purpose: prove local search.

Criteria:

- FTS5 search works.
- Embedding generation works.
- `sqlite-vec` index works.
- RRF ranking works.
- CLI search returns useful results.

### 0.4.0-linux-tui

Purpose: prove interactive UX.

Criteria:

- Basic PTY proxy works.
- `ratatui` overlay works.
- Terminal restore behavior is acceptable.
- Search result selection inserts or prepares the selected command.

### 0.5.0-linux-alpha

Purpose: early public Linux alpha.

Criteria:

- Installer script exists.
- Core docs exist.
- Performance budget is measured.
- Security tests cover redaction and permissions.
- Known limitations are documented.

### 0.6.0-windows-powershell-preview

Purpose: begin Windows support through the PAL.

Criteria:

- Named Pipe IPC works.
- PowerShell capture works.
- Windows paths and permissions are correct.
- Core database and search code remain shared.

### 1.0.0

Purpose: stable local-first semantic history release.

Criteria:

- Linux support is reliable.
- PowerShell support is usable or clearly marked preview.
- Search quality and performance are stable.
- Security posture is documented.
- No cloud dependency exists.
- Architecture boundaries remain intact.

## Deferred Future Work

The following ideas are valid long-term directions but must not distract from the MVP:

- Ghost-text shell autosuggestions.
- Fish and Nushell integrations.
- SQLCipher encryption.
- Local generative assistant workflows.
- Git and filesystem memory graph expansion.
- Enterprise knowledge graph features.
- Team-level sharing with strict privacy controls.

These features should only begin after the local Linux semantic history engine is reliable, tested, and releasable.
