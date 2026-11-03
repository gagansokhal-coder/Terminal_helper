# Autonomous Repository Management System

This directory documents the autonomous repository-management and contributor-orchestration system deployed in this repository.

The system is designed to operate autonomously within **GitHub Actions** and supported **GitHub Copilot** integrations, operating independently without requiring external AI sessions to remain connected.

---

## What Runs Automatically

1. **Continuous Health Inspection:**
   - Daily and weekly verification runs defined in `.github/repository-automation/config.json`.
   - Runs formatting, linting, tests, and builds against trusted code.
   - Generates and uploads full JSON and Markdown inspection reports as workflow artifacts.
2. **Contributor Assignment Automation:**
   - Detects `/assign` comments on issues bearing eligible labels (`good first issue`, `beginner`, `intermediate`, `advanced`, `NSOC`).
   - Serialized per-issue queue to prevent race conditions.
   - Re-fetches live issue status and checks GitHub assignee eligibility before writing.
   - Enforces a 1-active-issue quota per contributor.
3. **Automated Contributor Maintenance:**
   - Weekly run detects inactive assignments (7 days without activity: friendly reminder; 14 days without activity: safe unassignment).
4. **Pull Request Policy & Security Feedback:**
   - Detects missing issue references and posts advisory checklists.
   - Warns maintainers immediately if sensitive files (workflows, install scripts, Dockerfiles) are touched.
5. **Copilot CLI Maintenance Analysis:**
   - Optional weekly automated code analysis using GitHub Copilot CLI in Actions.
   - Verified for users with GitHub Student Developer Pack Copilot entitlements via `COPILOT_GITHUB_TOKEN`.
6. **Branch-Protection-Aware Auto-Merge Gate:**
   - Enforces strict conditions (passing CI, 1 approval, zero conflicts, no sensitive file modifications) before requesting GitHub squash auto-merge.

---

## Safety Guardrails & Default-Off Controls

The system follows a strict principle of least privilege and zero unverified writes:

- **Issue Creation:** Disabled by default (`REPOSITORY_AUTOMATION_ISSUE_WRITES=false`). Inspection runs produce dry-run report artifacts until you choose to enable automatic issue filing.
- **Auto-Merge:** Disabled by default (`REPOSITORY_AUTOMATION_AUTOMERGE=false`). Cannot merge PRs or bypass branch protection rules.
- **Copilot Analysis:** Report-only artifact generation (`copilot-maintenance-report`). Does not push code or open PRs.

---

## One-Step Universal Bootstrap for ANY Repository

You can deploy this entire automation suite to **any** other repository in one command:

### 1. Cross-Platform Node.js (Universal)
```bash
node tools/repository-automation/bootstrap.mjs --target /path/to/any-repo
```

### 2. Linux / macOS Bash
```bash
./tools/repository-automation/bootstrap.sh -t /path/to/any-repo
```

### 3. Windows PowerShell
```powershell
pwsh -File .\tools\repository-automation\bootstrap.ps1 -TargetPath C:\path\to\any-repo
```

### Flags Available:
- `--commit, -c`: Staged and commit the bootstrap files on the new branch automatically.
- `--without-copilot`: Install pure GitHub Actions workflows only (excludes Copilot files).
- `--force, -f`: Overwrite existing automation files if re-running.
- `--branch, -b <name>`: Custom branch name (default: `automation-bootstrap`).

The bootstrapper automatically detects whether the target project uses **Rust (`Cargo.toml`)**, **Node.js (`package.json`)**, **Python (`pyproject.toml`/`setup.py`)**, **Go (`go.mod`)**, or **Java/Gradle (`pom.xml`/`build.gradle`)**, generates the matching inspection commands in `.github/repository-automation/config.json`, and stages all files on a new branch ready for review.

---

## Documentation Links

- [Setup & Configuration Guide](setup.md)
- [Security Model & Permissions Matrix](security-model.md)
- [Operating Runbook & Troubleshooting](runbook.md)
- [Project Status & Merged Milestones](../PROJECT-STATUS.md)
- [Agent Handoff Guide](../AGENT-HANDOFF.md)
