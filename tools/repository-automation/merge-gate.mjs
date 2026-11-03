import { readFile } from 'node:fs/promises';
import { githubClient, truthy } from './lib/common.mjs';

if (!truthy(process.env.REPOSITORY_AUTOMATION_AUTOMERGE)) {
  console.log('Auto-merge is disabled by repository policy.');
  process.exit(0);
}
const event = JSON.parse(await readFile(process.env.GITHUB_EVENT_PATH, 'utf8'));
const config = JSON.parse(await readFile('.github/repository-automation/config.json', 'utf8'));
const pr = event.pull_request;
if (!pr || pr.draft || pr.mergeable_state === 'dirty') {
  console.log('PR is draft or conflicted; no auto-merge request made.');
  process.exit(0);
}
const client = githubClient({ token: process.env.GITHUB_TOKEN, repository: process.env.GITHUB_REPOSITORY });
const details = await client.request(`/repos/${client.repository}/pulls/${pr.number}`);
if (!details.mergeable || details.draft) {
  console.log('PR is not currently mergeable.');
  process.exit(0);
}
const requiredIssue = /(?:close[sd]?|fix(?:e[sd])?|resolve[sd]?)\s+#\d+/i.test(`${details.title}\n${details.body ?? ''}`);
if (!requiredIssue) {
  console.log('PR has no closing issue reference; no auto-merge request made.');
  process.exit(0);
}
const files = await client.request(`/repos/${client.repository}/pulls/${pr.number}/files?per_page=100`);
const sensitive = files
  .map((file) => file.filename)
  .filter((file) => config.pull_requests.sensitive_path_prefixes.some((prefix) => file === prefix || file.startsWith(prefix)));
if (sensitive.length) {
  console.log(`PR touches configured sensitive paths (${sensitive.join(', ')}); maintainer approval is required and auto-merge was not requested.`);
  process.exit(0);
}
const result = await client.request('/graphql', {
  method: 'POST',
  headers: { 'content-type': 'application/json' },
  body: JSON.stringify({
    query: `mutation EnableAutoMerge($pullRequestId: ID!) {
      enablePullRequestAutoMerge(input: {pullRequestId: $pullRequestId, mergeMethod: SQUASH}) {
        pullRequest { number autoMergeRequest { enabledAt } }
      }
    }`,
    variables: { pullRequestId: details.node_id },
  }),
});
if (result.errors?.length) throw new Error(`GitHub could not enable auto-merge: ${result.errors.map((error) => error.message).join('; ')}`);
console.log(`Requested GitHub squash auto-merge for #${pr.number}; branch protection remains authoritative.`);
