import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';

const SECRET_PATTERNS = [
  /gh[pousr]_[A-Za-z0-9_]{16,}/g,
  /github_pat_[A-Za-z0-9_]{16,}/g,
  /(api[_-]?key|token|password|secret)\s*[=:]\s*[^\s]+/gi,
];

export async function loadConfig(path = '.github/repository-automation/config.json') {
  const parsed = JSON.parse(await readFile(path, 'utf8'));
  if (parsed.version !== 1 || !Array.isArray(parsed.inspection?.commands)) {
    throw new Error(`Unsupported automation configuration at ${path}`);
  }
  return parsed;
}

export function fingerprint(value) {
  return createHash('sha256').update(value).digest('hex').slice(0, 20);
}

export function redact(value = '') {
  let safe = String(value);
  for (const pattern of SECRET_PATTERNS) safe = safe.replace(pattern, '[REDACTED]');
  return safe;
}

export function tail(value = '', limit = 4000) {
  const safe = redact(value);
  return safe.length <= limit ? safe : `…${safe.slice(-limit)}`;
}

export function truthy(value) {
  return String(value).toLowerCase() === 'true';
}

export function hasAssignableLabel(labels, allowed) {
  const names = new Set(labels.map((label) => typeof label === 'string' ? label : label.name));
  return allowed.some((label) => names.has(label));
}

export function isAssignmentRequest(body = '') {
  return /(^|\s)\/assign\b/i.test(body) || /\b(i(?:'d| would) like to work on (?:this|it)|i want to work on (?:this|it)|can i work on (?:this|it))\b/i.test(body);
}

export function issueMarker(id) {
  return `<!-- repository-automation:fingerprint=${id} -->`;
}

export function hasDuplicateSearchResult(search) {
  return Number(search?.total_count) > 0;
}

export function assignmentMarker(login, timestamp) {
  return `<!-- repository-automation:assignment assignee=${login} assigned_at=${timestamp} -->`;
}

export function buildIssueBody(candidate) {
  return `${issueMarker(candidate.fingerprint)}
## Verified problem

The scheduled repository inspection ran **${candidate.name}** and the command failed.

### Reproduce

\`\`\`text
${candidate.command.join(' ')}
\`\`\`

Exit code: \`${candidate.exitCode}\`

### Evidence (redacted, final output)

\`\`\`text
${candidate.output || '(no output captured)'}
\`\`\`

## Expected outcome

The command succeeds on the default branch and the relevant regression is covered by an automated test where practical.

## Suggested implementation

Identify the underlying failing component, make the smallest safe correction, and update tests or documentation as needed. Do not commit terminal history, credentials, or machine-specific data.

## Acceptance criteria

- [ ] The reproduction command exits successfully.
- [ ] A regression test is added or the reason one is not practical is documented.
- [ ] Existing formatting, lint, test, and build checks pass.
- [ ] No sensitive data is included in the pull request.

## Contribution metadata

- Difficulty: **intermediate**
- Priority: **high**
- NSOC suitability: **yes, after maintainer confirms the failure is reproducible**
`;
}

export function githubClient({ token, repository, fetchImpl = fetch }) {
  if (!token) throw new Error('GITHUB_TOKEN is required');
  if (!repository?.includes('/')) throw new Error('GITHUB_REPOSITORY must be owner/repository');

  async function request(path, options = {}) {
    const response = await fetchImpl(`https://api.github.com${path}`, {
      ...options,
      headers: {
        accept: 'application/vnd.github+json',
        authorization: `Bearer ${token}`,
        'x-github-api-version': '2026-03-10',
        ...(options.headers ?? {}),
      },
    });
    if (response.status === 204) return null;
    const body = await response.json().catch(() => ({}));
    if (!response.ok) {
      const error = new Error(`GitHub API ${options.method ?? 'GET'} ${path} failed (${response.status}): ${body.message ?? 'unknown error'}`);
      error.status = response.status;
      throw error;
    }
    return body;
  }

  return { request, repository };
}
