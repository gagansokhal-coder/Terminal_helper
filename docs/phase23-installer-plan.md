# Phase 23 - Installer & Distribution (Completed)

> *Note: This phase established the Linux/WSL distribution pathways. Windows native installers were subsequently implemented in Phase 26.*

## Goals

The primary goal of Phase 23 is to provide a seamless, frictionless installation experience for users. This includes:

* **One-line installation**: Users should be able to install `ggnmem` with a single command.
* **Automatic platform detection**: The installer must detect the user's OS and architecture.
* **Automatic latest release detection**: Dynamically resolve and pull the latest stable version from GitHub Releases.
* **Checksum verification**: Ensure integrity and security of the downloaded binary bundle.
* **Shell integration**: Automatically configure Bash and Zsh hooks.
* **Daemon startup**: Start the `ggnmem` background service as part of the installation process.
* **Minimal user interaction**: The script should operate autonomously with sensible defaults, requiring minimal to no interactive prompts.

## User Flow

The installation sequence will follow this automated pipeline:

```mermaid
flowchart TD
    A[curl | bash] --> B[Detect platform]
    B --> C[Find latest release]
    C --> D[Download bundle]
    D --> E[Verify checksum]
    E --> F[Extract]
    F --> G[Install binaries]
    G --> H[Setup config]
    H --> I[Setup shell integration]
    I --> J[Start daemon]
    J --> K[Run doctor]
```

## Platform Support

The initial rollout of the installer will officially support:

* Linux x86_64
* Linux aarch64
* WSL (Windows Subsystem for Linux)
* Future macOS support (planned, architecture in place)

## Download Strategy

The installer will utilize a robust download strategy relying on existing GitHub infrastructure:

* **GitHub Releases as source of truth**: No external mirrors or hosting required.
* **Use GitHub API to discover latest version**: Parse the latest release tags to determine the bundle version.
* **Select correct asset automatically**: Match the detected platform and architecture to the correct `.tar.gz` asset in the release.

## Verification Strategy

Security and integrity are paramount. The installer will enforce:

* **SHA256 verification**: Download `checksums.txt` and verify the bundle hash before extraction.
* **Abort on mismatch**: If the hash does not perfectly match the expected SHA256 checksum, the installer will delete the downloaded artifact.
* **No installation if verification fails**: System binaries will not be touched unless verification succeeds.

## Shell Integration

To ensure `ggnmem` correctly captures terminal history out of the box:

* **Bash support**: Inject `PROMPT_COMMAND` hooks into `.bashrc`.
* **Zsh support**: Inject `preexec`/`precmd` hooks into `.zshrc`.
* **Existing shell hooks preserved**: Safely append integration blocks without disrupting or overwriting the user's existing shell configuration.

## Rollback Strategy

The installer must be safe and idempotent, protecting the user's environment:

* **Backup binaries before install**: Existing `ggnmem` and `ggnmem-daemon` binaries are moved to `.old` suffixes.
* **Restore backups on failure**: If any step of the installation fails (e.g., extraction or shell integration), the old binaries are restored.
* **Preserve config/database/models**: User data, history, and ML models are intentionally left untouched during binary updates.

## Future Improvements

While the `curl | bash` script serves as the foundational MVP, future distribution mechanisms should include:

* Homebrew tap for macOS and Linux users
* apt repository for Debian/Ubuntu distributions
* Hosted redirect via `install.ggnmem.dev` for a cleaner curl command
* Native Windows installer (MSI or winget)
* Auto-update channel selection (stable vs. nightly builds)
