# Operating Runbook

## Routine operation

1. Review the daily or weekly **Repository Automation — Inspect** summary.
2. Download its `repository-inspection-report` artifact when a check fails.
3. Before enabling issue writes, use **Run workflow** with `publish_issues=false`
   and confirm the candidates are genuine.
4. With issue writes enabled, review new `NSOC` issues periodically. A generated
   issue contains evidence, reproducible command, acceptance criteria, and an
   automation fingerprint.

## Contributor assignment

Contributors can comment `/assign` on an issue with an eligible difficulty or
`NSOC` label. One open automation-managed assignment per contributor is allowed
by default. The workflow replies with either an assignment confirmation or a
clear reason it declined. Maintainers can modify the policy in
`.github/repository-automation/config.json`.

The weekly maintenance workflow reminds assigned contributors after seven days
without issue activity and removes the assignee after fourteen days only after a
warning. It never penalizes a contributor for limited GitHub history.

## Pull requests

The PR policy workflow posts a single maintained comment. It identifies missing
issue references and marks changes under sensitive paths for maintainer review.
It is advisory; CI and branch protection remain the merge authority.

## Recovery

- To pause writes, set `REPOSITORY_AUTOMATION_ISSUE_WRITES=false`.
- To pause all scheduled analysis, disable the corresponding workflow in the
  Actions tab.
- To stop assignment replies, set `assignment.enabled_by_default` to `false`
  and merge the config change.
- To revoke Copilot access, delete the Actions secret and set the Copilot
  variable to `false`.
