# Repository Automation Status

Last reviewed: <!-- YYYY-MM-DD -->

## Current position

Repository Automation is installed but operates conservatively by default:

- Scheduled inspection produces reports and artifacts.
- Issue creation is disabled until a maintainer enables it after a dry run.
- Copilot analysis is disabled until its credentials and plan are verified.
- Auto-merge is disabled until protected-branch requirements are configured and
  a maintainer explicitly authorizes it.

## Required owner actions

1. Review the bootstrap branch through a pull request.
2. Run label initialization once.
3. Run a manual inspection and review its artifact.
4. Configure branch protection before enabling auto-merge.

See `docs/repository-automation/` for the runbook and security model.
