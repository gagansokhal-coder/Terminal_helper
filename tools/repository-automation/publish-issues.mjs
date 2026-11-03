import { readFile } from 'node:fs/promises';
import { buildIssueBody, githubClient, hasDuplicateSearchResult, issueMarker, truthy } from './lib/common.mjs';

const reportPath = process.env.AUTOMATION_REPORT_PATH ?? 'automation-output/inspection-report.json';
const config = JSON.parse(await readFile('.github/repository-automation/config.json', 'utf8'));
const report = JSON.parse(await readFile(reportPath, 'utf8'));
const enabled = truthy(process.env.REPOSITORY_AUTOMATION_ISSUE_WRITES) && truthy(process.env.PUBLISH_ISSUES);

if (!enabled) {
  console.log('Issue publication is disabled. Set both the repository variable and workflow input to true after reviewing a dry run.');
  process.exit(0);
}

const client = githubClient({ token: process.env.GITHUB_TOKEN, repository: process.env.GITHUB_REPOSITORY });
let created = 0;
for (const candidate of report.candidates.slice(0, config.issues.max_per_run)) {
  const query = encodeURIComponent(`repo:${client.repository} in:body "${issueMarker(candidate.fingerprint)}"`);
  const search = await client.request(`/search/issues?q=${query}&per_page=1`);
  if (hasDuplicateSearchResult(search)) {
    console.log(`Skipping duplicate finding ${candidate.fingerprint}`);
    continue;
  }
  const issue = await client.request(`/repos/${client.repository}/issues`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ title: candidate.title, body: buildIssueBody(candidate), labels: config.issues.labels }),
  });
  created += 1;
  console.log(`Created issue #${issue.number}: ${issue.html_url}`);
}
console.log(`Issue publication complete: ${created} created.`);
