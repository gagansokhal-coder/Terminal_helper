# Agent Handoff & Architecture Manual

> **Purpose:** This document is the master architectural handoff for any AI coding assistant (Antigravity, Claude, Codex, Copilot, etc.) or human developer working on this repository. Read this file first to understand the system layout, merged milestones, security rules, and outstanding objectives.

---

## 1. System Identity & Mission

- **Repository:** `gagansokhal-coder/Terminal_helper` (`ggnmem`)
- **Default Branch:** `main`
- **Objective:** Maintain an **Autonomous GitHub Repository Management & NSOC Contributor System** that runs independently inside GitHub Actions and supported GitHub Copilot integrations without requiring an external AI chat session to remain active.

---

## 2. Repository Layout

```
.
├── .github/
│   ├── ISSUE_TEMPLATE/                   # Contributor issue templates
│   ├── PULL_REQUEST_TEMPLATE.md          # Standard PR checklist
│   ├── copilot-instructions.md           # Instructions for GitHub Copilot in PR reviews & editor
│   ├── repository-automation/
│   │   ├── config.json                   # Central automation configuration & inspection rules
│   │   └── schema.json                   # JSON schema for config.json
│   └── workflows/
│       ├── ci.yml                        # Primary CI: Cargo format, clippy, test, release build
│       ├── release.yml                   # Release packager for Linux and Windows binaries
│       ├── repository-automation-inspect.yml       # Health inspection & verified issue discovery
│       ├── repository-automation-labels.yml        # Taxonomy label initializer
│       ├── repository-automation-assignment.yml    # Contributor /assign comment handler
│       ├── repository-automation-pr-policy.yml     # PR policy check & sensitive file warning
│       ├── repository-automation-maintenance.yml   # Stale contributor reminders & unassignment
│       ├── repository-automation-merge-gate.yml    # Safe opt-in squash auto-merge gate
│       └── repository-automation-copilot.yml       # Copilot CLI automated maintenance analysis
├── docs/
│   ├── PROJECT-STATUS.md                 # Up-to-date position, merged PRs, and next owner actions
│   ├── AGENT-HANDOFF.md                  # This file
│   ├── architecture.md                   # Core ggnmem system architecture
│   ├── roadmap.md                        # Product development roadmap
│   └── repository-automation/            # Complete operator and security documentation
│       ├── README.md                     # Technical automation overview & cross-platform guide
│       ├── setup.md                      # Setup guide, secrets, variables, and permissions
│       ├── runbook.md                    # Day-to-day operations and recovery procedures
│       └── security-model.md             # Least privilege security matrix
├── tools/
│   └── repository-automation/            # Standalone Node.js & script automation engine
│       ├── bootstrap.mjs                 # Cross-platform 1-step bootstrapper for ANY repository
│       ├── bootstrap.sh                  # Bash wrapper for Linux / macOS / WSL
│       ├── bootstrap.ps1                 # PowerShell wrapper for Windows / PowerShell Core
│       ├── inspect.mjs                   # Health inspector & check runner
│       ├── publish-issues.mjs            # Deduplicating issue publisher
│       ├── assign-on-comment.mjs         # Safe contributor assignment logic
│       ├── maintain-assignments.mjs      # Stale assignment reminder & cleanup logic
│       ├── pr-policy.mjs                 # PR policy evaluation & advisory commenting
│       ├── initialize-labels.mjs         # Label creator
│       ├── merge-gate.mjs                # Branch-protection-aware auto-merge requester
│       ├── lib/common.mjs                # Shared token redaction, regexes, and GitHub API client
│       ├── templates/                    # Bootstrapper templates
│       └── test/                         # Node.js automated unit test suite (12 tests)
└── ggnmem-*                              # Rust workspace crates (cli, daemon, db, ai, etc.)
```

---

## 3. Operational State & Guardrails Matrix

Every automated write is guarded to prevent accidental changes:

| Component | Default Setting | How to Activate | Safety Behavior |
| --- | --- | --- | --- |
| **Issue Creation** | `false` | Set repo variable `REPOSITORY_AUTOMATION_ISSUE_WRITES=true` | In dry-run mode, inspection generates full JSON & markdown artifacts without opening issues. |
| **Auto-Merge** | `false` | Set repo variable `REPOSITORY_AUTOMATION_AUTOMERGE=true` | Gate refuses to request auto-merge unless approved, required checks pass, no conflicts, and no sensitive files touched. |
| **Copilot Schedule** | `false` | Set repo variable `REPOSITORY_AUTOMATION_COPILOT=true` | Manual workflow runs are always available and verified. Recurring schedule runs only when toggled on. |
| **Copilot Secret** | Active | `COPILOT_GITHUB_TOKEN` | Fine-grained PAT with Copilot Requests permission from the owner's GitHub Student Developer Pack. |

---

## 4. Hardening & Security Guarantees

Any agent modifying this codebase MUST maintain these guarantees:
1. **Never commit directly to `main`:** Always create an isolated feature branch and open a PR.
2. **Never execute untrusted fork code in privileged workflows:** Pull requests from forks only run under `ci.yml` with `contents: read` and zero secrets.
3. **No secret exposure:** All script logs redact GitHub tokens, auth headers, and bearer tokens via `redactSecretString()` in `lib/common.mjs`.
4. **Issue deduplication:** Every generated issue embeds a unique SHA-256 fingerprint marker `<!-- repository-automation-fingerprint: <id> -->`. The publisher queries open and closed issues before opening a duplicate.
5. **No race condition in `/assign`:** Assignment runs are serialized with GitHub Actions `concurrency: group: repository-automation-assignment-${{ issue.number }}`. The script re-fetches the live issue from GitHub to prevent stale comment-event snapshots.
6. **Assignee eligibility check:** Before assigning, the system queries `/repos/{owner}/{repo}/assignees/{username}`. If GitHub reports the contributor is ineligible, an instructional comment is posted instead of failing the workflow.

---

## 5. How to Run Automation Tests Locally

From the repository root:

```bash
# Run all automation unit tests (12 tests covering tokens, regexes, assignment, bootstrapper):
node tools/repository-automation/test/common.test.mjs
node tools/repository-automation/test/assignment.test.mjs
node tools/repository-automation/test/bootstrap.test.mjs

# Syntax and lint checks:
node --check tools/repository-automation/bootstrap.mjs
node --check tools/repository-automation/inspect.mjs
node --check tools/repository-automation/publish-issues.mjs
node --check tools/repository-automation/assign-on-comment.mjs
node --check tools/repository-automation/maintain-assignments.mjs
node --check tools/repository-automation/pr-policy.mjs
node --check tools/repository-automation/merge-gate.mjs
node --check tools/repository-automation/lib/common.mjs
```

---

## 6. How to Deploy to ANY Other Repository in One Step

The automation suite is completely decoupled from `Terminal_helper`. To install it on any other project (Rust, Node, Python, Go, Java):

```bash
# Universal (runs on any OS with Node 18+):
node tools/repository-automation/bootstrap.mjs --target /path/to/target-repository

# Linux / macOS Bash:
./tools/repository-automation/bootstrap.sh -t /path/to/target-repository

# Windows PowerShell:
pwsh -File .\tools\repository-automation\bootstrap.ps1 -TargetPath C:\path\to\target-repository
```

Options:
- `--commit`: Automatically stages and commits the installation files on the generated `automation-bootstrap` branch.
- `--without-copilot`: Installs pure GitHub Actions workflows only (excluding Copilot CLI analysis).
- `--force`: Overwrites existing files if re-running.

---

## 7. Current Project Position Summary

- **Merged PRs:** #13, #14, #15, #16, #17 all merged clean into `main`.
- **CI Health:** 100% green on `main`.
- **Copilot Action Run:** Run #41601254578 succeeded and produced `copilot-maintenance-report`.
- **Next User Action:** Configure GitHub branch protection on `main` before turning on `REPOSITORY_AUTOMATION_AUTOMERGE=true`.
