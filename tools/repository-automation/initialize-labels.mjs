import { githubClient } from './lib/common.mjs';

const labels = [
  ['good first issue', '7057ff', 'Approachable contribution for newcomers'],
  ['beginner', '0e8a16', 'Suitable for a beginner contributor'],
  ['intermediate', 'fbca04', 'Requires familiarity with the project'],
  ['advanced', 'b60205', 'Requires substantial project expertise'],
  ['bug', 'd73a4a', 'A verified defect'],
  ['enhancement', 'a2eeef', 'Improvement to existing behavior'],
  ['documentation', '0075ca', 'Documentation improvement'],
  ['performance', 'f9d0c4', 'Performance-related work'],
  ['security', 'ee0701', 'Security-related work'],
  ['testing', 'bfdadc', 'Testing or verification work'],
  ['NSOC', '5319e7', 'Suitable for NSOC contribution'],
  ['status: in progress', '1d76db', 'An assigned contribution is in progress'],
];
const client = githubClient({ token: process.env.GITHUB_TOKEN, repository: process.env.GITHUB_REPOSITORY });
const existing = await client.request(`/repos/${client.repository}/labels?per_page=100`);
const names = new Set(existing.map((label) => label.name));
for (const [name, color, description] of labels) {
  if (names.has(name)) continue;
  await client.request(`/repos/${client.repository}/labels`, {
    method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ name, color, description }),
  });
  console.log(`Created label: ${name}`);
}
