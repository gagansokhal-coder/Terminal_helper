# Security Model

## Trust boundaries

| Workflow class | Trigger | Token | Untrusted code executed? |
| --- | --- | --- | --- |
| CI | `pull_request` | `contents: read` | Yes, with no secrets or write permissions. |
| Inspection | schedule/manual/default-branch push | scoped read token; issue write only in publisher | Only trusted default-branch code. |
| Assignment/PR policy | comments and PR metadata | issue/PR write only | No. These workflows never check out a contributor branch. |
| Merge gate | PR metadata | PR write only | No. |

No workflow uses a personal Git token for routine repository operations.
`GITHUB_TOKEN` is short-lived and scoped in every workflow. The optional
Copilot credential is exposed only to the Copilot process and is never used to
write code, issues, or pull requests.

## Safety rules

- Inspection reports static observations but never turns TODO/FIXME text alone
  into an issue. Only a failed configured verification command is a candidate.
- The publisher searches open and closed issues and pull requests for its
  fingerprint before creating an issue.
- Maximum issue creation per run is configured and defaults to three.
- Assignment requests for the same issue are serialized. Each queued run then
  fetches the live issue state before checking that it is open, labelled for
  assignment, unassigned, and that the requester is below the configured
  open-assignment limit before writing.
- Comment-triggered workflows treat all issue and comment text as untrusted
  data. They do not interpolate it into shell commands.
- Fork pull-request code runs only in the existing read-only CI workflow.
- No privileged workflow checks out, builds, tests, or installs a fork PR.
- Auto-merge is disabled by default and honors branch protection.
- Copilot analysis is disabled by default and produces an artifact only; it
  cannot publish issues, comments, pull requests, or merges.

## Required response to an incident

Immediately set all `REPOSITORY_AUTOMATION_*` variables to `false`, revoke
`COPILOT_GITHUB_TOKEN` if present, and inspect recent workflow runs and the
audit log. Do not delete evidence. Open a private security report following
[`SECURITY.md`](../../SECURITY.md).
