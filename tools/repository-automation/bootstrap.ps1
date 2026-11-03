[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateNotNullOrEmpty()]
    [string]$TargetPath,
    [string]$BranchName = 'automation-bootstrap',
    [switch]$WithoutCopilot,
    [switch]$Force
)

$ErrorActionPreference = 'Stop'
$toolkitRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$targetRoot = (git -C $TargetPath rev-parse --show-toplevel).Trim()
if (-not $targetRoot) { throw 'TargetPath must be inside a Git repository.' }

function Copy-AutomationFile {
    param([string]$RelativePath)
    $source = Join-Path $toolkitRoot $RelativePath
    $destination = Join-Path $targetRoot $RelativePath
    if ((Test-Path $destination) -and -not $Force) {
        throw "Refusing to overwrite existing file: $destination. Review it or rerun with -Force."
    }
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $destination) | Out-Null
    Copy-Item -LiteralPath $source -Destination $destination -Force
}

function Get-InspectionCommands {
    if (Test-Path (Join-Path $targetRoot 'Cargo.toml')) {
        return @(
            @{ id = 'format'; name = 'Formatting check'; argv = @('cargo', 'fmt', '--all', '--', '--check') },
            @{ id = 'clippy'; name = 'Lint check'; argv = @('cargo', 'clippy', '--workspace', '--all-targets', '--', '-D', 'warnings') },
            @{ id = 'test'; name = 'Workspace tests'; argv = @('cargo', 'test', '--workspace') },
            @{ id = 'build'; name = 'Release build'; argv = @('cargo', 'build', '--release', '--workspace') }
        )
    }
    if (Test-Path (Join-Path $targetRoot 'package.json')) {
        return @(
            @{ id = 'test'; name = 'Package tests'; argv = @('npm', 'test', '--', '--if-present') },
            @{ id = 'build'; name = 'Package build'; argv = @('npm', 'run', 'build', '--if-present') }
        )
    }
    if (Test-Path (Join-Path $targetRoot 'pyproject.toml')) {
        return @(@{ id = 'test'; name = 'Python tests'; argv = @('python', '-m', 'pytest') })
    }
    if (Test-Path (Join-Path $targetRoot 'go.mod')) {
        return @(@{ id = 'test'; name = 'Go tests'; argv = @('go', 'test', './...') })
    }
    throw 'No supported project marker found (Cargo.toml, package.json, pyproject.toml, or go.mod). Add inspection.commands manually after installation.'
}

if ((git -C $targetRoot status --porcelain)) { throw 'Target repository has uncommitted changes. Commit or stash them before bootstrap.' }
if (git -C $targetRoot show-ref --verify --quiet "refs/heads/$BranchName") { throw "Branch already exists: $BranchName" }
git -C $targetRoot checkout -b $BranchName

$files = @(
    '.github\workflows\repository-automation-inspect.yml',
    '.github\workflows\repository-automation-labels.yml',
    '.github\workflows\repository-automation-assignment.yml',
    '.github\workflows\repository-automation-pr-policy.yml',
    '.github\workflows\repository-automation-maintenance.yml',
    '.github\workflows\repository-automation-merge-gate.yml',
    '.github\repository-automation\schema.json',
    'tools\repository-automation\inspect.mjs',
    'tools\repository-automation\publish-issues.mjs',
    'tools\repository-automation\assign-on-comment.mjs',
    'tools\repository-automation\maintain-assignments.mjs',
    'tools\repository-automation\pr-policy.mjs',
    'tools\repository-automation\initialize-labels.mjs',
    'tools\repository-automation\merge-gate.mjs',
    'tools\repository-automation\lib\common.mjs'
)
if (-not $WithoutCopilot) {
    $files += @(
        '.github\copilot-instructions.md',
        '.github\workflows\repository-automation-copilot.yml'
    )
}
foreach ($file in $files) { Copy-AutomationFile $file }

$docsSource = Join-Path $toolkitRoot 'docs\repository-automation'
$docsTarget = Join-Path $targetRoot 'docs\repository-automation'
if ((Test-Path $docsTarget) -and -not $Force) { throw "Refusing to overwrite existing directory: $docsTarget" }
Copy-Item -Recurse -Force $docsSource $docsTarget
Copy-Item (Join-Path $toolkitRoot 'tools\repository-automation\templates\PROJECT-AUTOMATION-STATUS.md') (Join-Path $targetRoot 'docs\PROJECT-AUTOMATION-STATUS.md') -Force

try {
    $defaultBranch = (git -C $targetRoot symbolic-ref --short refs/remotes/origin/HEAD).Trim().Replace('origin/', '')
} catch {
    $defaultBranch = (git -C $targetRoot branch --show-current).Trim()
}
if (-not $defaultBranch) { $defaultBranch = 'main' }
$config = @{
    version = 1
    project = @{ name = (Split-Path $targetRoot -Leaf); default_branch = $defaultBranch }
    inspection = @{ max_issue_candidates = 3; commands = @(Get-InspectionCommands) }
    issues = @{ enabled_by_default = $false; labels = @('bug', 'testing', 'NSOC'); max_per_run = 3 }
    assignment = @{ enabled_by_default = $true; max_open_assignments_per_contributor = 1; assignable_labels = @('good first issue', 'beginner', 'intermediate', 'advanced', 'NSOC'); reminder_after_days = 7; unassign_after_days = 14 }
    pull_requests = @{ sensitive_path_prefixes = @('.github/workflows/', '.github/repository-automation/', 'scripts/install', 'install.sh', 'install.ps1', 'Dockerfile') }
}
$configPath = Join-Path $targetRoot '.github\repository-automation\config.json'
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $configPath) | Out-Null
$config | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $configPath -Encoding utf8

Write-Host "Installed Repository Automation on branch '$BranchName'."
if ($WithoutCopilot) { Write-Host 'Copilot files were excluded; this installation uses GitHub Actions only.' }
Write-Host 'Review the changes, run tests, then commit and open a pull request. No commit, push, repository setting, issue, or merge was performed.'
