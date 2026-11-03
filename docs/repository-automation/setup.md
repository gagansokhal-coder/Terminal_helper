# Setup & Configuration Guide

This guide walks you through setting up and configuring the autonomous repository management system for this repository or any new repository bootstrapped with the toolkit.

---

## 1. Prerequisites & Repository Settings

In your GitHub repository under **Settings → Actions → General**:
1. **Workflow permissions:** Set to **Read and write permissions** so that explicitly scoped jobs can manage issues and PR comments.
2. **Pull request creation:** Keep **Allow GitHub Actions to create and approve pull requests** *unchecked* (disabled) for least privilege.

---

## 2. Repository Variables (Guardrails)

Configure these repository variables under **Settings → Secrets and variables → Actions → Variables**:

| Variable Name | Recommended Initial Value | Purpose & Effect |
| --- | --- | --- |
| `REPOSITORY_AUTOMATION_ISSUE_WRITES` | `false` | When `false`, health inspections run in dry-run mode and generate report artifacts. Set to `true` to allow verified failures to open GitHub issues automatically. |
| `REPOSITORY_AUTOMATION_AUTOMERGE` | `false` | When `false`, the auto-merge gate is dormant. Keep `false` until branch protection is configured and owner approves. |
| `REPOSITORY_AUTOMATION_COPILOT` | `false` | Controls the weekly scheduled Copilot CLI analysis run. Set to `true` after verifying a manual run. |

---

## 3. GitHub Copilot Configuration (Student Developer Pack & Pro)

### For Personal Repositories (GitHub Student Developer Pack / Pro / Individual):
GitHub Student Developer Pack includes personal Copilot access. In a personal repository, GitHub Actions runs Copilot CLI by providing a personal access token with Copilot permissions:

1. Go to **GitHub Settings → Developer settings → Personal access tokens → Fine-grained tokens**.
2. Create a token scoped to your user account.
3. Under **Permissions**, grant:
   - **Copilot Requests:** Read (or Read and Write).
4. Do NOT grant broad repository admin, code write, or secret permissions.
5. In your repository, go to **Settings → Secrets and variables → Actions → Secrets**.
6. Add a new repository secret:
   - Name: `COPILOT_GITHUB_TOKEN`
   - Secret: `<your-fine-grained-copilot-token>`
7. Test the integration: Go to **Actions → Repository Automation — Copilot analysis → Run workflow**.
8. Confirm the run completes successfully and downloads the `copilot-maintenance-report` artifact (as verified in run `#41601254578`).

### For Organization-Owned Repositories:
If the repository is transferred to an organization with Copilot Business/Enterprise, an owner can check **Allow use of Copilot CLI billed to the organization** in org settings. The workflow will then automatically use the default `GITHUB_TOKEN` without requiring a stored secret.

---

## 4. Initializing Taxonomy Labels

After merging the automation branch into your default branch (`main`):
1. Navigate to the **Actions** tab in GitHub.
2. Select **Repository Automation — Initialize labels**.
3. Click **Run workflow**.
4. This idempotently creates all necessary labels:
   - `good first issue`, `beginner`, `intermediate`, `advanced`
   - `bug`, `enhancement`, `documentation`, `performance`, `security`, `testing`
   - `NSOC`, `status: in progress`

---

## 5. Setting up Branch Protection for `main`

Before setting `REPOSITORY_AUTOMATION_AUTOMERGE=true`:
1. Go to **Settings → Rules → Rulesets** (or **Settings → Branches**).
2. Create a ruleset targeting `main`.
3. Configure the following rules:
   - **Require a pull request before merging:** Require at least 1 approval; dismiss stale approvals when new commits are pushed.
   - **Require status checks to pass:** Add required status checks:
     - `Formatting`
     - `Clippy`
     - `Tests`
     - `Build (release)`
     - `PR policy`
   - **Require conversation resolution:** Enabled.
   - **Block force pushes:** Enabled.
   - **Block branch deletions:** Enabled.
   - **Do not allow bypass:** Ensure no users or apps can bypass the rules.

---

## 6. One-Step Deployment to Another Repository

To automate another repository:
```bash
# On any OS:
node tools/repository-automation/bootstrap.mjs --target /path/to/another-repo

# Then in the target repository:
git push -u origin automation-bootstrap
# Open PR, review, merge, and initialize labels!
```
