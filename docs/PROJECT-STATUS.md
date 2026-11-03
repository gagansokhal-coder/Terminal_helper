# Project Status

Last reviewed: 2026-11-03

## Current state

`ggnmem` is an active Rust pre-alpha semantic terminal-memory engine. The
workspace is split into focused crates for the CLI, daemon, database, local AI,
knowledge base, model management, paths, and PTY integration. The existing CI
checks formatting, Clippy, tests, and a release build; the release workflow
builds Linux and Windows artifacts from version tags.

The project is local-first: command history and embeddings stay on the user’s
machine. Contributors must not commit command histories, secrets, personal
data, or machine-specific configuration.

## Automation position

The `automation-bootstrap` branch adds a reusable GitHub repository-management
layer without changing `main`. It adds conservative health inspection,
evidence-backed issue candidates, contributor assignment, PR policy feedback,
label initialization, optional Copilot CLI analysis, and a disabled-by-default
auto-merge gate. Operational detail is in
[Repository Automation](repository-automation/README.md).

## Next owner actions

1. Review the automation branch through a pull request.
2. Run the label-initialization workflow after merge.
3. Run inspection manually with issue publication disabled and verify reports.
4. Configure branch protection before considering auto-merge.
5. Enable issue writes or Copilot only through the documented repository
   variables after controlled testing.
