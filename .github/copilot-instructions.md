# Repository Automation Instructions

This repository is a Rust workspace for a local-first terminal-memory engine.
Treat terminal history, command output, tokens, paths, and user data as
sensitive. Never place real command history, credentials, or machine-specific
information in source, issues, pull requests, or workflow output.

When asked to review or automate work:

1. Read `docs/PROJECT-STATUS.md`, `CONTRIBUTING.md`, `SECURITY.md`, and the
   relevant crate before proposing changes.
2. Respect the workspace boundaries: CLI, daemon, database, local AI,
   knowledge, model, paths, and PTY code remain separate.
3. Prefer the smallest change that includes appropriate tests and documentation.
4. Never claim a command, test, or security check passed unless its recorded
   result shows it passed.
5. Do not create an issue from a TODO, FIXME, or speculative observation alone.
   Evidence from a reproducible check is required.
6. Do not change workflow permissions, protected-branch settings, secrets, or
   release configuration without an explicit maintainer request.
7. Never merge, force-push, or push directly to `main`.

For repository automation, follow `.github/repository-automation/config.json`
and the controls documented in `docs/repository-automation/`.
