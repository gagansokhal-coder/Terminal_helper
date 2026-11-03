import { readFile } from 'node:fs/promises';
import { githubClient } from './lib/common.mjs';

const event = JSON.parse(await readFile(process.env.GITHUB_EVENT_PATH, 'utf8'));
const config = JSON.parse(await readFile('.github/repository-automation/config.json', 'utf8'));
const pr = event.pull_request;
if (!pr || pr.state !== 'open') process.exit(0);

const client = githubClient({ token: process.env.GITHUB_TOKEN, repository: process.env.GITHUB_REPOSITORY });
const files = await client.request(`/repos/${client.repository}/pulls/${pr.number}/files?per_page=100`);
const changed = files.map((file) => file.filename);
const sensitive = changed.filter((file) => config.pull_requests.sensitive_path_prefixes.some((prefix) => file === prefix || file.startsWith(prefix)));
const linkedIssue = /(?:close[sd]?|fix(?:e[sd])?|resolve[sd]?)\s+#\d+|#\d+/i.test(`${pr.title}\n${pr.body ?? ''}`);
const marker = '<!-- repository-automation:pr-policy -->';
const checks = [
  `${linkedIssue ? '✅' : '⚠️'} ${linkedIssue ? 'Related issue reference found.' : 'Please link the pull request to its issue (for example, `Fixes #123`).'}`,
  `${sensitive.length ? `⚠️ Maintainer review required for sensitive paths: ${sensitive.map((file) => `\`${file}\``).join(', ')}` : '✅ No configured sensitive paths changed.'}`,
  'ℹ️ Existing CI is the source of truth for formatting, lint, tests, and build status.',
];
const body = `${marker}\n## Repository automation policy check\n\n${checks.join('\n\n')}\n\nThis is advisory. It does not approve, merge, or bypass branch protection.`;
const comments = await client.request(`/repos/${client.repository}/issues/${pr.number}/comments?per_page=100`);
const existing = comments.find((comment) => comment.body?.includes(marker));
if (existing) {
  await client.request(`/repos/${client.repository}/issues/comments/${existing.id}`, {
    method: 'PATCH', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ body }),
  });
} else {
  await client.request(`/repos/${client.repository}/issues/${pr.number}/comments`, {
    method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ body }),
  });
}
console.log(`Published PR policy feedback for #${pr.number}.`);
