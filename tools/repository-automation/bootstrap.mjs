#!/usr/bin/env node

/**
 * Universal One-Step Repository Automation Bootstrapper
 * 
 * Works across Windows, Linux, and macOS.
 * Installs the complete, tested GitHub Actions & Copilot automation suite
 * into any target Git repository with a single command.
 * 
 * Usage:
 *   node tools/repository-automation/bootstrap.mjs --target <path-to-target-repo> [options]
 * 
 * Options:
 *   --target, -t <path>      Path to the target Git repository (required)
 *   --branch, -b <name>      Branch name to create (default: automation-bootstrap)
 *   --without-copilot        Install GitHub Actions only (exclude Copilot workflow & instructions)
 *   --force, -f              Overwrite existing automation files if present
 *   --commit, -c             Automatically stage and commit the bootstrap files on the new branch
 *   --help, -h               Show this help message
 */

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const toolkitRoot = path.resolve(__dirname, '..', '..');

function parseArgs(args) {
  const options = {
    target: '',
    branch: 'automation-bootstrap',
    withoutCopilot: false,
    force: false,
    commit: false,
    help: false
  };

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === '--help' || arg === '-h') {
      options.help = true;
    } else if (arg === '--target' || arg === '-t') {
      options.target = args[++i] || '';
    } else if (arg.startsWith('--target=')) {
      options.target = arg.slice('--target='.length);
    } else if (arg === '--branch' || arg === '-b') {
      options.branch = args[++i] || options.branch;
    } else if (arg.startsWith('--branch=')) {
      options.branch = arg.slice('--branch='.length);
    } else if (arg === '--without-copilot') {
      options.withoutCopilot = true;
    } else if (arg === '--force' || arg === '-f') {
      options.force = true;
    } else if (arg === '--commit' || arg === '-c') {
      options.commit = true;
    }
  }

  return options;
}

function printHelp() {
  console.log(`
Universal One-Step Repository Automation Bootstrapper

Usage:
  node tools/repository-automation/bootstrap.mjs --target <path> [options]

Options:
  --target, -t <path>      Path to the target Git repository (required)
  --branch, -b <name>      Branch name to create (default: automation-bootstrap)
  --without-copilot        Install GitHub Actions only (exclude Copilot files)
  --force, -f              Overwrite existing automation files if present
  --commit, -c             Automatically git commit changes on target branch
  --help, -h               Show this help message

Examples:
  node tools/repository-automation/bootstrap.mjs --target ../my-app
  node tools/repository-automation/bootstrap.mjs -t /path/to/repo --commit
  node tools/repository-automation/bootstrap.mjs -t C:\\src\\project --without-copilot
`);
}

function runGit(cwd, args) {
  try {
    return execFileSync('git', args, { cwd, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
  } catch (error) {
    const stderr = error.stderr ? error.stderr.trim() : error.message;
    throw new Error(`Git command failed (git ${args.join(' ')}): ${stderr}`);
  }
}

function detectInspectionCommands(targetRoot) {
  // 1. Rust
  if (fs.existsSync(path.join(targetRoot, 'Cargo.toml'))) {
    return [
      { id: 'format', name: 'Formatting check', argv: ['cargo', 'fmt', '--all', '--', '--check'] },
      { id: 'clippy', name: 'Lint check', argv: ['cargo', 'clippy', '--workspace', '--all-targets', '--', '-D', 'warnings'] },
      { id: 'test', name: 'Workspace tests', argv: ['cargo', 'test', '--workspace'] },
      { id: 'build', name: 'Release build', argv: ['cargo', 'build', '--release', '--workspace'] }
    ];
  }

  // 2. Node.js / JavaScript / TypeScript
  if (fs.existsSync(path.join(targetRoot, 'package.json'))) {
    const packageJsonPath = path.join(targetRoot, 'package.json');
    let hasTestScript = false;
    let hasBuildScript = false;
    let hasLintScript = false;

    try {
      const pkg = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
      hasTestScript = Boolean(pkg.scripts && pkg.scripts.test);
      hasBuildScript = Boolean(pkg.scripts && pkg.scripts.build);
      hasLintScript = Boolean(pkg.scripts && pkg.scripts.lint);
    } catch {
      // ignore parse errors, default to npm run with if-present
    }

    const commands = [];
    if (hasLintScript) {
      commands.push({ id: 'lint', name: 'Lint check', argv: ['npm', 'run', 'lint'] });
    }
    commands.push({ id: 'test', name: 'Package tests', argv: ['npm', 'run', 'test', '--if-present'] });
    if (hasBuildScript) {
      commands.push({ id: 'build', name: 'Package build', argv: ['npm', 'run', 'build', '--if-present'] });
    }
    return commands;
  }

  // 3. Python (pyproject.toml, setup.py, requirements.txt)
  if (fs.existsSync(path.join(targetRoot, 'pyproject.toml')) ||
      fs.existsSync(path.join(targetRoot, 'setup.py')) ||
      fs.existsSync(path.join(targetRoot, 'requirements.txt'))) {
    return [
      { id: 'test', name: 'Python tests', argv: ['python', '-m', 'pytest'] }
    ];
  }

  // 4. Go
  if (fs.existsSync(path.join(targetRoot, 'go.mod'))) {
    return [
      { id: 'test', name: 'Go tests', argv: ['go', 'test', './...'] }
    ];
  }

  // 5. Java / Gradle / Maven
  if (fs.existsSync(path.join(targetRoot, 'pom.xml'))) {
    return [
      { id: 'test', name: 'Maven tests', argv: ['mvn', 'test'] }
    ];
  }
  if (fs.existsSync(path.join(targetRoot, 'build.gradle')) || fs.existsSync(path.join(targetRoot, 'build.gradle.kts'))) {
    const gradlew = process.platform === 'win32' ? 'gradlew.bat' : './gradlew';
    return [
      { id: 'test', name: 'Gradle tests', argv: [gradlew, 'test'] }
    ];
  }

  // Fallback generic inspection command
  return [
    { id: 'status', name: 'Repository status check', argv: ['git', 'status', '--short'] }
  ];
}

function copyFileSafe(sourcePath, targetPath, force) {
  if (fs.existsSync(targetPath) && !force) {
    throw new Error(`Refusing to overwrite existing file: ${targetPath}. Use --force to overwrite.`);
  }
  fs.mkdirSync(path.dirname(targetPath), { recursive: true });
  fs.copyFileSync(sourcePath, targetPath);
}

function copyDirRecursive(sourceDir, targetDir, force) {
  if (!fs.existsSync(sourceDir)) return;
  fs.mkdirSync(targetDir, { recursive: true });
  const entries = fs.readdirSync(sourceDir, { withFileTypes: true });
  for (const entry of entries) {
    const srcPath = path.join(sourceDir, entry.name);
    const dstPath = path.join(targetDir, entry.name);
    if (entry.isDirectory()) {
      copyDirRecursive(srcPath, dstPath, force);
    } else {
      copyFileSafe(srcPath, dstPath, force);
    }
  }
}

export function bootstrapRepository(options) {
  if (!options.target) {
    throw new Error('Target repository path is required. Use --target <path>.');
  }

  const targetPathResolved = path.resolve(options.target);
  if (!fs.existsSync(targetPathResolved)) {
    throw new Error(`Target path does not exist: ${targetPathResolved}`);
  }

  let targetRoot;
  try {
    targetRoot = runGit(targetPathResolved, ['rev-parse', '--show-toplevel']);
  } catch {
    throw new Error(`Target path is not inside a Git repository: ${targetPathResolved}`);
  }

  const status = runGit(targetRoot, ['status', '--porcelain']);
  if (status && !options.force) {
    throw new Error('Target repository has uncommitted changes. Commit or stash them, or pass --force.');
  }

  const branchName = options.branch || 'automation-bootstrap';

  // Check if branch exists
  let branchExists = false;
  try {
    runGit(targetRoot, ['show-ref', '--verify', '--quiet', `refs/heads/${branchName}`]);
    branchExists = true;
  } catch {
    branchExists = false;
  }

  if (branchExists && !options.force) {
    throw new Error(`Branch already exists: ${branchName}. Choose a different name with --branch or use --force.`);
  }

  // Checkout or switch branch
  if (branchExists) {
    runGit(targetRoot, ['checkout', branchName]);
  } else {
    runGit(targetRoot, ['checkout', '-b', branchName]);
  }

  // Core files to install
  const filesToCopy = [
    '.github/workflows/repository-automation-inspect.yml',
    '.github/workflows/repository-automation-labels.yml',
    '.github/workflows/repository-automation-assignment.yml',
    '.github/workflows/repository-automation-pr-policy.yml',
    '.github/workflows/repository-automation-maintenance.yml',
    '.github/workflows/repository-automation-merge-gate.yml',
    '.github/repository-automation/schema.json',
    'tools/repository-automation/inspect.mjs',
    'tools/repository-automation/publish-issues.mjs',
    'tools/repository-automation/assign-on-comment.mjs',
    'tools/repository-automation/maintain-assignments.mjs',
    'tools/repository-automation/pr-policy.mjs',
    'tools/repository-automation/initialize-labels.mjs',
    'tools/repository-automation/merge-gate.mjs',
    'tools/repository-automation/lib/common.mjs'
  ];

  if (!options.withoutCopilot) {
    filesToCopy.push(
      '.github/copilot-instructions.md',
      '.github/workflows/repository-automation-copilot.yml'
    );
  }

  for (const relativePath of filesToCopy) {
    const src = path.join(toolkitRoot, relativePath);
    const dst = path.join(targetRoot, relativePath);
    if (!fs.existsSync(src)) {
      throw new Error(`Toolkit source file not found: ${src}`);
    }
    copyFileSafe(src, dst, options.force);
  }

  // Documentation directory copy
  const docsSource = path.join(toolkitRoot, 'docs', 'repository-automation');
  const docsTarget = path.join(targetRoot, 'docs', 'repository-automation');
  copyDirRecursive(docsSource, docsTarget, options.force);

  // Template status file
  const templateStatus = path.join(toolkitRoot, 'tools', 'repository-automation', 'templates', 'PROJECT-AUTOMATION-STATUS.md');
  const targetStatus = path.join(targetRoot, 'docs', 'PROJECT-AUTOMATION-STATUS.md');
  if (fs.existsSync(templateStatus)) {
    copyFileSafe(templateStatus, targetStatus, options.force);
  }

  // Detect default branch
  let defaultBranch = 'main';
  try {
    const remoteHead = runGit(targetRoot, ['symbolic-ref', '--short', 'refs/remotes/origin/HEAD']);
    defaultBranch = remoteHead.replace('origin/', '').trim();
  } catch {
    try {
      defaultBranch = runGit(targetRoot, ['branch', '--show-current']).trim() || 'main';
    } catch {
      defaultBranch = 'main';
    }
  }

  // Detect inspection commands based on language / framework
  const detectedCommands = detectInspectionCommands(targetRoot);
  const projectName = path.basename(targetRoot);

  const config = {
    version: 1,
    project: {
      name: projectName,
      default_branch: defaultBranch
    },
    inspection: {
      max_issue_candidates: 3,
      commands: detectedCommands
    },
    issues: {
      enabled_by_default: false,
      labels: ['bug', 'testing', 'NSOC'],
      max_per_run: 3
    },
    assignment: {
      enabled_by_default: true,
      max_open_assignments_per_contributor: 1,
      assignable_labels: ['good first issue', 'beginner', 'intermediate', 'advanced', 'NSOC'],
      reminder_after_days: 7,
      unassign_after_days: 14
    },
    pull_requests: {
      sensitive_path_prefixes: [
        '.github/workflows/',
        '.github/repository-automation/',
        'scripts/install',
        'install.sh',
        'install.ps1',
        'Dockerfile'
      ]
    }
  };

  const configPath = path.join(targetRoot, '.github', 'repository-automation', 'config.json');
  fs.mkdirSync(path.dirname(configPath), { recursive: true });
  fs.writeFileSync(configPath, JSON.stringify(config, null, 2) + '\n', 'utf8');

  // Optional auto-commit
  if (options.commit) {
    runGit(targetRoot, ['add', '.github', 'tools/repository-automation', 'docs/repository-automation', 'docs/PROJECT-AUTOMATION-STATUS.md']);
    runGit(targetRoot, ['commit', '-m', 'feat: add autonomous repository management system']);
  }

  return {
    targetRoot,
    branchName,
    withoutCopilot: options.withoutCopilot,
    detectedCommands,
    committed: options.commit
  };
}

// CLI execution
if (process.argv[1] && path.resolve(process.argv[1]) === path.resolve(__filename)) {
  const options = parseArgs(process.argv.slice(2));

  if (options.help || !options.target) {
    printHelp();
    process.exit(options.help ? 0 : 1);
  }

  try {
    const result = bootstrapRepository(options);
    console.log(`\n✔ Successfully installed Repository Automation in: ${result.targetRoot}`);
    console.log(`✔ Switched to branch: ${result.branchName}`);
    if (result.withoutCopilot) {
      console.log('ℹ Copilot files were excluded (--without-copilot specified).');
    } else {
      console.log('✔ Copilot instructions and workflow included.');
    }
    console.log(`✔ Detected ${result.detectedCommands.length} verification command(s) for project inspection.`);
    if (result.committed) {
      console.log('✔ Staged and committed changes on the branch.');
    } else {
      console.log('ℹ Files staged on working tree (not committed yet).');
    }
    console.log('\nNext steps:');
    console.log(`1. Review the changes on branch '${result.branchName}'.`);
    console.log('2. Push branch and open a Pull Request to merge into your default branch.');
    console.log('3. Run "Repository Automation — Initialize labels" once after merge.');
    console.log('4. See docs/repository-automation/setup.md for full details.\n');
  } catch (error) {
    console.error(`\n✖ Bootstrap error: ${error.message}\n`);
    process.exit(1);
  }
}
