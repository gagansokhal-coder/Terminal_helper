# Repository Automation

This directory documents the repository-management automation added in the
`automation-bootstrap` change. It is intentionally conservative: inspection is
automatic, while actions that change repository state require an explicit
repository variable or a maintainer action.

## What runs automatically

- Daily repository inspection and a weekly deeper inspection.
- Rust formatting, lint, test, and release-build checks defined in
  `.github/repository-automation/config.json`.
- A GitHub Actions job summary and a retained inspection report artifact.
- Assignment handling when a contributor comments `/assign` or says they would
  like to work on an eligible issue.
- Pull-request policy feedback, including a warning for sensitive files and a
  reminder to link an issue.

## What is intentionally off until configured

- Creating GitHub issues from verified failed checks.
- Copilot CLI analysis.
- Enabling auto-merge on a pull request.

See [Setup](setup.md), [Security model](security-model.md), and
[Operating runbook](runbook.md) before enabling those capabilities.

## Reuse in another repository

The automation is configuration-driven. Run the bootstrap script from this
repository against another local Git checkout:

```powershell
pwsh -File .\tools\repository-automation\bootstrap.ps1 -TargetPath C:\src\another-repository
```

For a GitHub Actions-only installation with no Copilot workflow or Copilot
instructions, add `-WithoutCopilot`:

```powershell
pwsh -File .\tools\repository-automation\bootstrap.ps1 -TargetPath C:\src\another-repository -WithoutCopilot
```

The script copies only the automation files, refuses to overwrite existing
files unless `-Force` is supplied, and creates a separate branch in the target
repository. It never commits, pushes, changes the default branch, or changes
repository settings.

After reviewing the generated branch, configure the repository settings in
[Setup](setup.md). This is the single repeatable installation step for another
repository; project-specific commands then live in that repository's config.
