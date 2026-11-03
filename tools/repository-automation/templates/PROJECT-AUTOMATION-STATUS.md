# Repository Automation Status

> **Target Project:** <!-- Project Name -->> **Status:** Installed on `automation-bootstrap`> **Operational Mode:** Conservative (Dry-Run by Default)

---

## Current Position

Repository Automation is installed and operates under strict least-privilege guardrails:

- **Scheduled & Manual Inspection:** Checks defined in `.github/repository-automation/config.json` run automatically and upload report artifacts.
- **Issue Creation:** Disabled by default (`REPOSITORY_AUTOMATION_ISSUE_WRITES=false`). Runs produce inspection reports without opening GitHub issues until activated.
- **Contributor Management:** Automatically processes `/assign` comments on issues with eligible labels (`good first issue`, `beginner`, `intermediate`, `advanced`, `NSOC`). Enforces single-assignment quotas and checks assignee eligibility.
- **Pull Request Policy:** Advisory checks run on open/synchronized PRs to warn if sensitive workflow or script files are modified.
- **Copilot CLI Maintenance:** Report-only analysis available via `repository-automation-copilot.yml`.
- **Auto-Merge:** Dormant by default (`REPOSITORY_AUTOMATION_AUTOMERGE=false`). Cannot merge PRs or bypass branch protection rules.

---

## Required Maintainer Actions

1. **Review and Merge Bootstrap PR:**
   - Review changes on the `automation-bootstrap` branch and merge into your default branch via Pull Request.
2. **Initialize Taxonomy Labels:**
   - In GitHub Actions, trigger **Repository Automation — Initialize labels** once.
3. **Run a Dry-Run Inspection:**
   - In GitHub Actions, trigger **Repository Automation — Inspect** manually and inspect the generated `repository-inspection-report` artifact.
4. **Configure Branch Protection:**
   - Require pull requests, passing CI checks, and at least 1 approval before enabling auto-merge.
5. **Set Repository Variables (Settings → Secrets and variables → Actions → Variables):**
   - `REPOSITORY_AUTOMATION_ISSUE_WRITES`: Set `true` to enable automated issue creation from verified failures.
   - `REPOSITORY_AUTOMATION_COPILOT`: Set `true` to enable weekly scheduled Copilot CLI maintenance.
   - `REPOSITORY_AUTOMATION_AUTOMERGE`: Set `true` only after branch protection is active.

See `docs/repository-automation/` for full operational runbooks and security model documentation.
