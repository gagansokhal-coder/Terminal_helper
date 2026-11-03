import { readFile } from 'node:fs/promises';
import { assignmentMarker, githubClient, hasAssignableLabel, isAssignmentRequest } from './lib/common.mjs';

const event = JSON.parse(await readFile(process.env.GITHUB_EVENT_PATH, 'utf8'));
const config = JSON.parse(await readFile('.github/repository-automation/config.json', 'utf8'));
const eventIssue = event.issue;
const actor = event.comment?.user?.login;
const request = event.comment?.body ?? '';

if (!config.assignment.enabled_by_default || !eventIssue || !actor || !isAssignmentRequest(request)) {
  console.log('Comment is not an enabled assignment request.');
  process.exit(0);
}

const client = githubClient({ token: process.env.GITHUB_TOKEN, repository: process.env.GITHUB_REPOSITORY });
// An issue_comment payload is a point-in-time snapshot. Fetch the issue again
// after GitHub has serialized this issue's workflow run, so a prior /assign is
// observed before we decide whether this requester can be assigned.
const issue = await client.request(`/repos/${client.repository}/issues/${eventIssue.number}`);
const comment = async (body) => client.request(`/repos/${client.repository}/issues/${issue.number}/comments`, {
  method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ body }),
});

if (issue.pull_request) {
  await comment('Thanks for offering to help. Assignment requests belong on the linked issue, not on a pull request.');
  process.exit(0);
}
if (issue.state !== 'open') {
  await comment('Thanks for your interest. This issue is not open for assignment.');
  process.exit(0);
}
if (issue.assignees?.length) {
  await comment('Thanks for your interest. This issue already has a primary assignee, so I have left the assignment unchanged.');
  process.exit(0);
}
if (!hasAssignableLabel(issue.labels ?? [], config.assignment.assignable_labels)) {
  await comment(`Thanks for offering to help. A maintainer must first label this issue as one of: ${config.assignment.assignable_labels.join(', ')}.`);
  process.exit(0);
}

const query = encodeURIComponent(`repo:${client.repository} is:issue is:open assignee:${actor}`);
const assigned = await client.request(`/search/issues?q=${query}&per_page=100`);
if (assigned.total_count >= config.assignment.max_open_assignments_per_contributor) {
  await comment(`Thanks for offering to help. To keep assignments fair, the current policy allows ${config.assignment.max_open_assignments_per_contributor} open assigned issue per contributor. Please finish or ask a maintainer to unassign an existing task first.`);
  process.exit(0);
}

await client.request(`/repos/${client.repository}/issues/${issue.number}/assignees`, {
  method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ assignees: [actor] }),
});
try {
  await client.request(`/repos/${client.repository}/issues/${issue.number}/labels`, {
    method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ labels: ['status: in progress'] }),
  });
} catch (error) {
  console.warn(`Assignment succeeded, but the status label could not be applied: ${error.message}`);
}
await comment(`Assigned to @${actor}. Thank you! Please create a focused branch, link your pull request to #${issue.number}, include tests where practical, and do not include credentials, terminal history, or private data.\n\n${assignmentMarker(actor, new Date().toISOString())}`);
console.log(`Assigned #${issue.number} to ${actor}.`);
