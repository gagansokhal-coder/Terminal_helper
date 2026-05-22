# Development Environment

## Decision

Chosen strategy: **Option C - Hybrid local + Docker**.

`ggnmem` should be developed locally first, with Docker available as a reproducible Linux development environment. This matches the project architecture: the real product is a local-first terminal tool with shell hooks, a daemon, SQLite storage, and a PTY layer. Docker is only a contributor and CI-style toolchain wrapper.

## Options Evaluated

### Option A - Pure Local Development

Pros:

- Best fit for shell hooks, PTY behavior, terminal integration, and OS-specific runtime testing.
- Fastest edit-build-test loop when the host toolchain is healthy.
- Keeps the developer close to the real environment where the daemon and hooks will eventually run.

Cons:

- Host setup drift can block contributors.
- Windows requires MSVC Build Tools or a working 64-bit GNU toolchain for native SQLite C compilation.
- Harder to guarantee identical Linux-first checks across machines.

### Option B - Docker DevContainer

Pros:

- Reproducible Linux toolchain.
- Consistent Rust, SQLite, build tools, rustfmt, and clippy.
- Useful for contributors who do not want to install a full local Rust stack first.

Cons:

- Not suitable for validating real shell hook behavior.
- Not suitable for validating the eventual PTY proxy against a real user shell session.
- Can hide platform-specific filesystem, permission, and terminal edge cases.

### Option C - Hybrid Local + Docker

Pros:

- Local development remains the source of truth for runtime behavior.
- Docker provides a clean Linux-first validation path.
- Contributors can start quickly while still moving to local testing for shell, daemon, and PTY phases.
- Avoids treating containers as part of the product architecture.

Cons:

- Maintainers must keep local and Docker setup instructions aligned.
- Some tests will eventually need explicit labels for local-only versus container-safe execution.

## Environment Boundary

Docker is selected only for development and validation.

Docker must not be used to containerize:

- the daemon runtime architecture
- shell hooks
- PTY behavior
- user IPC paths
- production deployment

Future runtime work must still target the documented architecture: local process boundaries, Unix Domain Sockets on Linux, Named Pipes on Windows, local SQLite, and local model execution.

## Tooling Included

The Docker development image includes:

- Rust toolchain
- Cargo
- rustfmt
- clippy
- SQLite CLI and development headers
- build-essential
- pkg-config
- clang
- cmake
- lld
- gdb
- git
- curl

## Local Workflow

Use local development when working on:

- shell integration
- IPC permissions
- daemon lifecycle
- PTY behavior
- OS-specific filesystem paths
- performance and latency measurements

Recommended local commands:

```bash
cargo check --workspace --all-targets
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

On Windows, native builds require a working MSVC toolchain with `link.exe`, or a working 64-bit GNU toolchain capable of compiling SQLite C sources. Until that is available, WSL or Docker is the recommended validation path.

## Docker Workflow

Build the development image:

```bash
docker compose -f docker-compose.dev.yml build
```

Start the development container:

```bash
docker compose -f docker-compose.dev.yml up -d
```

Run checks:

```bash
docker compose -f docker-compose.dev.yml exec dev cargo check --workspace --all-targets
docker compose -f docker-compose.dev.yml exec dev cargo test --workspace
docker compose -f docker-compose.dev.yml exec dev cargo fmt --all -- --check
docker compose -f docker-compose.dev.yml exec dev cargo clippy --workspace --all-targets -- -D warnings
```

Open a shell:

```bash
docker compose -f docker-compose.dev.yml exec dev bash
```

Stop the container:

```bash
docker compose -f docker-compose.dev.yml down
```

## DevContainer Workflow

Editors that support Dev Containers can open the repository through `.devcontainer/devcontainer.json`.

The DevContainer uses the same `docker-compose.dev.yml` service as the command-line Docker workflow. It mounts the repository at `/workspace` and keeps Cargo registry, Git, and target directories in named Docker volumes.

## Contributor Onboarding

1. Read the project architecture docs:

   - `docs/project.md`
   - `docs/architecture.md`
   - `docs/roadmap.md`
   - `docs/agent_memory.md`
   - `docs/skills.md`

2. Choose a development path:

   - Use local Rust for normal implementation work.
   - Use Docker or DevContainer for reproducible Linux checks.
   - Use WSL on Windows when native MSVC or 64-bit GNU tooling is unavailable.

3. Run the validation commands before submitting changes:

```bash
cargo check --workspace --all-targets
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

4. Keep architecture boundaries intact:

   - no cloud APIs
   - no alternative database
   - no new runtime stack
   - no monolithic process shortcut
   - no containerized product runtime

## Current Validation Baseline

The expected validation baseline for this phase is:

- local Linux or WSL build passes
- Docker image builds
- Docker workspace checks pass
- formatting and clippy are available in both local and Docker workflows

Native Windows validation remains dependent on installing a working Windows Rust C build toolchain.
