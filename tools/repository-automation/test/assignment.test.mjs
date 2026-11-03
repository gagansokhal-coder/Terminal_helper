import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import test from 'node:test';
import { pathToFileURL } from 'node:url';
import { spawn } from 'node:child_process';

const repositoryRoot = resolve(import.meta.dirname, '../../..');
const assignmentScript = join(repositoryRoot, 'tools/repository-automation/assign-on-comment.mjs');

function event({ actor = 'student', assignees = [] } = {}) {
  return {
    issue: { number: 41, state: 'open', assignees, labels: [{ name: 'NSOC' }] },
    comment: { user: { login: actor }, body: '/assign' },
  };
}

function currentIssue({ assignees = [] } = {}) {
  return { number: 41, state: 'open', assignees, labels: [{ name: 'NSOC' }] };
}

async function runAssignment({ payload, current, searchCount = 0 }) {
  const directory = await mkdtemp(join(tmpdir(), 'assignment-test-'));
  const eventPath = join(directory, 'event.json');
  const preloadPath = join(directory, 'mock-fetch.mjs');
  const callsPath = join(directory, 'calls.json');
  await writeFile(eventPath, JSON.stringify(payload));
  await writeFile(preloadPath, `
    import { writeFileSync } from 'node:fs';
    const calls = [];
    const current = ${JSON.stringify(current)};
    globalThis.fetch = async (url, options = {}) => {
      const method = options.method ?? 'GET';
      calls.push({ url: String(url), method, body: options.body ?? null });
      const body = String(url).includes('/search/issues') ? { total_count: ${searchCount} }
        : (method === 'GET' && /\\/issues\\/41$/.test(String(url))) ? current
        : { number: 1, html_url: 'https://example.invalid/1' };
      return new Response(JSON.stringify(body), { status: 200, headers: { 'content-type': 'application/json' } });
    };
    process.on('exit', () => writeFileSync(${JSON.stringify(callsPath)}, JSON.stringify(calls)));
  `);

  try {
    await new Promise((resolveRun, rejectRun) => {
      const child = spawn(process.execPath, [
        '--import', pathToFileURL(preloadPath).href,
        '--input-type=module',
        '--eval', 'await import(process.env.ASSIGNMENT_SCRIPT)',
      ], {
        cwd: repositoryRoot,
        env: {
          ...process.env,
          ASSIGNMENT_SCRIPT: pathToFileURL(assignmentScript).href,
          GITHUB_EVENT_PATH: eventPath,
          GITHUB_TOKEN: 'test-token',
          GITHUB_REPOSITORY: 'example/repository',
        },
        stdio: 'ignore',
      });
      child.on('error', rejectRun);
      child.on('close', (code) => code === 0 ? resolveRun() : rejectRun(new Error(`assignment script exited ${code}`)));
    });
    return JSON.parse(await readFile(callsPath, 'utf8'));
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

const writesToAssignees = (calls) => calls.filter((call) => call.method === 'POST' && call.url.includes('/assignees'));

test('uses the current issue state to reject an already-assigned issue', async () => {
  const calls = await runAssignment({
    payload: event(),
    current: currentIssue({ assignees: [{ login: 'first-student' }] }),
  });
  assert.equal(calls.filter((call) => call.method === 'GET' && call.url.endsWith('/issues/41')).length, 1);
  assert.equal(writesToAssignees(calls).length, 0);
});

test('enforces the contributor assignment quota before assignment', async () => {
  const calls = await runAssignment({ payload: event(), current: currentIssue(), searchCount: 1 });
  assert.equal(writesToAssignees(calls).length, 0);
  assert.equal(calls.filter((call) => call.url.includes('/search/issues')).length, 1);
});

test('permits one eligible assignment', async () => {
  const calls = await runAssignment({ payload: event(), current: currentIssue() });
  assert.equal(writesToAssignees(calls).length, 1);
  assert.equal(calls.filter((call) => call.method === 'POST' && call.url.includes('/labels')).length, 1);
});

test('rejects a stale queued request when the issue was assigned by the prior run', async () => {
  const calls = await runAssignment({
    payload: event({ actor: 'second-student', assignees: [] }),
    current: currentIssue({ assignees: [{ login: 'first-student' }] }),
  });
  assert.equal(writesToAssignees(calls).length, 0);
});
