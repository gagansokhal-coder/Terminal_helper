import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { basename } from 'node:path';
import { fingerprint, loadConfig, tail } from './lib/common.mjs';

const outputPath = process.env.AUTOMATION_REPORT_PATH ?? 'automation-output/inspection-report.json';

function run(command) {
  const [file, ...args] = command.argv;
  const result = spawnSync(file, args, {
    encoding: 'utf8',
    timeout: 15 * 60 * 1000,
    windowsHide: true,
  });
  const combined = `${result.stdout ?? ''}${result.stderr ?? ''}`;
  return {
    id: command.id,
    name: command.name,
    command: command.argv,
    passed: !result.error && result.status === 0,
    skipped: result.error?.code === 'ENOENT',
    exitCode: result.status ?? 1,
    output: tail(result.error ? `${combined}\n${result.error.message}` : combined),
  };
}

async function staticObservations() {
  const files = ['README.md', 'CONTRIBUTING.md', 'SECURITY.md', 'docs/roadmap.md'];
  const observations = [];
  for (const file of files) {
    try {
      const contents = await readFile(file, 'utf8');
      const matches = contents.match(/\b(?:TODO|FIXME)\b/g)?.length ?? 0;
      if (matches) observations.push({ file, type: 'static-note', count: matches, action: 'reported only; not an issue candidate' });
    } catch {
      // Optional documentation may not exist in a reused repository.
    }
  }
  return observations;
}

const config = await loadConfig();
const checks = config.inspection.commands.map(run);
const candidates = checks
  // A missing tool belongs to the runner/environment, not to the project. It
  // is reported for diagnosis but is never eligible for issue publication.
  .filter((check) => !check.passed && !check.skipped)
  .slice(0, config.issues.max_per_run)
  .map((check) => ({
    ...check,
    fingerprint: fingerprint(`${check.id}\n${check.command.join('\u0000')}\n${check.output}`),
    title: `Verified CI failure: ${check.name}`,
  }));

const report = {
  schemaVersion: 1,
  generatedAt: new Date().toISOString(),
  project: config.project.name,
  checks,
  candidates,
  observations: await staticObservations(),
  policy: 'Only failed configured verification commands are issue candidates. Static observations are never published as issues on their own.',
};

await mkdir('automation-output', { recursive: true });
await writeFile(outputPath, `${JSON.stringify(report, null, 2)}\n`);

console.log(`Repository inspection completed: ${checks.filter((check) => check.passed).length}/${checks.length} checks passed; ${candidates.length} issue candidates.`);
for (const check of checks) console.log(`${check.passed ? 'PASS' : check.skipped ? 'SKIP' : 'FAIL'} ${basename(check.command[0])}: ${check.name}`);
