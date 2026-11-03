import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { execFileSync } from 'node:child_process';
import { bootstrapRepository } from '../bootstrap.mjs';

test('universal bootstrapper installs automation suite into a target repository', (t) => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'repo-automation-bootstrap-test-'));

  try {
    // Initialize a git repo in tempDir
    execFileSync('git', ['init', '-b', 'main'], { cwd: tempDir, stdio: 'ignore' });
    execFileSync('git', ['config', 'user.name', 'Test User'], { cwd: tempDir, stdio: 'ignore' });
    execFileSync('git', ['config', 'user.email', 'test@example.com'], { cwd: tempDir, stdio: 'ignore' });

    // Create a mock package.json
    fs.writeFileSync(
      path.join(tempDir, 'package.json'),
      JSON.stringify({ name: 'sample-project', scripts: { test: 'echo test', build: 'echo build' } }, null, 2),
      'utf8'
    );
    execFileSync('git', ['add', '.'], { cwd: tempDir, stdio: 'ignore' });
    execFileSync('git', ['commit', '-m', 'chore: initial commit'], { cwd: tempDir, stdio: 'ignore' });

    // Run bootstrapper
    const result = bootstrapRepository({
      target: tempDir,
      branch: 'automation-bootstrap-test',
      withoutCopilot: false,
      force: false,
      commit: false
    });

    assert.equal(result.branchName, 'automation-bootstrap-test');
    assert.equal(result.withoutCopilot, false);

    // Verify workflows were copied
    const inspectWorkflow = path.join(tempDir, '.github', 'workflows', 'repository-automation-inspect.yml');
    assert.ok(fs.existsSync(inspectWorkflow), 'Inspection workflow must exist');

    const copilotWorkflow = path.join(tempDir, '.github', 'workflows', 'repository-automation-copilot.yml');
    assert.ok(fs.existsSync(copilotWorkflow), 'Copilot workflow must exist when withoutCopilot is false');

    // Verify config.json was generated
    const configPath = path.join(tempDir, '.github', 'repository-automation', 'config.json');
    assert.ok(fs.existsSync(configPath), 'config.json must exist');

    const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
    assert.equal(config.version, 1);
    assert.equal(config.issues.enabled_by_default, false);
    assert.ok(Array.isArray(config.inspection.commands));
    assert.ok(config.inspection.commands.some((c) => c.id === 'test'));

    // Verify docs were copied
    const docsReadme = path.join(tempDir, 'docs', 'repository-automation', 'README.md');
    assert.ok(fs.existsSync(docsReadme), 'docs README must exist');

    const statusDoc = path.join(tempDir, 'docs', 'PROJECT-AUTOMATION-STATUS.md');
    assert.ok(fs.existsSync(statusDoc), 'status document must exist');

    // Verify tools were copied
    const inspectTool = path.join(tempDir, 'tools', 'repository-automation', 'inspect.mjs');
    assert.ok(fs.existsSync(inspectTool), 'inspect.mjs must exist');
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('universal bootstrapper excludes copilot when --without-copilot is specified', (t) => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'repo-automation-bootstrap-nocopilot-'));

  try {
    execFileSync('git', ['init', '-b', 'main'], { cwd: tempDir, stdio: 'ignore' });
    execFileSync('git', ['config', 'user.name', 'Test User'], { cwd: tempDir, stdio: 'ignore' });
    execFileSync('git', ['config', 'user.email', 'test@example.com'], { cwd: tempDir, stdio: 'ignore' });

    fs.writeFileSync(path.join(tempDir, 'README.md'), '# Test\n', 'utf8');
    execFileSync('git', ['add', '.'], { cwd: tempDir, stdio: 'ignore' });
    execFileSync('git', ['commit', '-m', 'chore: initial commit'], { cwd: tempDir, stdio: 'ignore' });

    const result = bootstrapRepository({
      target: tempDir,
      branch: 'actions-only-test',
      withoutCopilot: true,
      force: false,
      commit: false
    });

    assert.equal(result.withoutCopilot, true);

    const copilotWorkflow = path.join(tempDir, '.github', 'workflows', 'repository-automation-copilot.yml');
    assert.ok(!fs.existsSync(copilotWorkflow), 'Copilot workflow must not exist when withoutCopilot is true');

    const copilotInstructions = path.join(tempDir, '.github', 'copilot-instructions.md');
    assert.ok(!fs.existsSync(copilotInstructions), 'Copilot instructions must not exist when withoutCopilot is true');

    const inspectWorkflow = path.join(tempDir, '.github', 'workflows', 'repository-automation-inspect.yml');
    assert.ok(fs.existsSync(inspectWorkflow), 'Inspection workflow must still exist');
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});
