# Setup

## 1. Review and merge the bootstrap pull request

Merge this automation only through a normal pull request. Do not push it to
`main` directly. The existing CI and release workflows are preserved.

## 2. Repository settings

In **Settings → Actions → General**, allow Actions to use the permissions
required by the workflows, then rely on each workflow's explicit minimal
`permissions` declaration. Enable **Issues**, **Actions**, and (optionally)
**Discussions** for the repository.

Create these repository variables, not secrets:

| Variable | Initial value | Effect |
| --- | --- | --- |
| `REPOSITORY_AUTOMATION_ISSUE_WRITES` | `false` | Allows verified inspection findings to open issues only when `true`. |
| `REPOSITORY_AUTOMATION_AUTOMERGE` | `false` | Enables the auto-merge gate only when `true`. Leave false until separately approved. |
| `REPOSITORY_AUTOMATION_COPILOT` | `false` | Enables the recurring optional Copilot CLI analysis schedule when `true`. Manual report-only runs remain available. |

## 3. Labels

Run **Repository Automation — Initialize labels** manually once after merging.
It creates the documented labels if absent: `good first issue`, `beginner`,
`intermediate`, `advanced`, `bug`, `enhancement`, `documentation`,
`performance`, `security`, `testing`, `NSOC`, and `status: in progress`.

## 4. Copilot (optional)

For an organization-owned repository, an owner can enable Copilot CLI billing
for Actions. The workflow then uses the short-lived `GITHUB_TOKEN` with
`copilot-requests: write` and requires no stored credential.

For a personal repository, add a repository Actions secret named
`COPILOT_GITHUB_TOKEN`. It must be a fine-grained GitHub token with only the
**Copilot Requests** account permission enabled, limited to this repository
wherever the GitHub token UI permits. Never use that token for Git writes and
never print it.

Run **Repository Automation — Copilot analysis** manually after adding the
secret. It is report-only and does not need the repository variable. Set
`REPOSITORY_AUTOMATION_COPILOT=true` only after that manual run succeeds and
you want the weekly schedule enabled.
Copilot cloud automations and automatic Copilot code review are separate paid
features; this optional workflow uses Copilot CLI in Actions instead.

## 5. Auto-merge (optional, owner approval required)

Keep `REPOSITORY_AUTOMATION_AUTOMERGE=false` unless the owner has explicitly
approved auto-merge. Before enabling it:

1. Protect `main`: require a pull request, passing required checks, resolved
   conversations, and at least one independent approval.
2. Disallow force pushes and branch-deletion bypasses.
3. Enable squash merging and GitHub auto-merge in repository settings.
4. Make the existing `CI / Formatting`, `CI / Clippy`, `CI / Tests`, and
   `CI / Build (release)` checks required.

The automation requests GitHub auto-merge only; it does not bypass a rule,
directly merge a pull request, or force-push.
