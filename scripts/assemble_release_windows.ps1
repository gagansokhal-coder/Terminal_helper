# ═══════════════════════════════════════════════════════════════════════════════
# assemble_release_windows.ps1 — Package Windows binaries into a release ZIP
#
# Called by the release workflow after `cargo build --release --target x86_64-pc-windows-msvc`.
# Produces: ggnmem-windows-x86_64.zip in the project root.
#
# Usage:
#   pwsh scripts/assemble_release_windows.ps1 `
#     -Target  x86_64-pc-windows-msvc `
#     -Arch    x86_64 `
#     -Version 0.4.0-alpha `
#     -Commit  abc1234 `
#     -Date    2026-06-20 `
#     -RustcVer 1.95.0
# ═══════════════════════════════════════════════════════════════════════════════

param(
    [Parameter(Mandatory)] [string] $Target,
    [Parameter(Mandatory)] [string] $Arch,
    [Parameter(Mandatory)] [string] $Version,
    [Parameter(Mandatory)] [string] $Commit,
    [Parameter(Mandatory)] [string] $Date,
    [Parameter(Mandatory)] [string] $RustcVer
)

$ErrorActionPreference = "Stop"

# ─── Paths ───────────────────────────────────────────────────────────────────

$ScriptDir   = Split-Path -Parent $MyInvocation.MyCommand.Definition
$ProjectRoot = Split-Path -Parent $ScriptDir
Set-Location $ProjectRoot

$BinaryDir  = "target\$Target\release"
$ReleaseDir = Join-Path $env:TEMP "ggnmem-assemble-$(Get-Random)"

# ─── Verify binaries exist ──────────────────────────────────────────────────

$CliBin    = Join-Path $BinaryDir "ggnmem-cli.exe"
$DaemonBin = Join-Path $BinaryDir "ggnmem-daemon.exe"

foreach ($bin in @($CliBin, $DaemonBin)) {
    if (-not (Test-Path $bin)) {
        Write-Error "ERROR: $bin not found"
        exit 1
    }
}

# ─── Assemble ────────────────────────────────────────────────────────────────

Write-Host "Assembling release for windows-${Arch}..."

New-Item -ItemType Directory -Path $ReleaseDir -Force | Out-Null

# Copy and rename CLI binary.
Copy-Item $CliBin (Join-Path $ReleaseDir "ggnmem.exe")
Write-Host "  ggnmem.exe"

# Copy daemon binary.
Copy-Item $DaemonBin (Join-Path $ReleaseDir "ggnmem-daemon.exe")
Write-Host "  ggnmem-daemon.exe"

# Generate VERSION file.
@"
version=$Version
commit=$Commit
date=$Date
arch=$Arch
rust=$RustcVer
platform=windows
"@ | Set-Content (Join-Path $ReleaseDir "VERSION") -NoNewline
Write-Host "  VERSION"

# Generate README.
@"
# ggnmem — Semantic Terminal Memory Engine

A local-first, privacy-focused terminal history intelligence system.

Website: https://ggnmem.mytechy.in

## Quick Install

``````powershell
irm https://ggnmem.mytechy.in/install.ps1 | iex
``````

## Upgrade

``````powershell
ggnmem self-update
``````

## Usage

``````powershell
# Search your command history
ggnmem search docker

# Interactive search
ggnmem ui

# Show recent commands
ggnmem recent

# Check health
ggnmem doctor

# Show version info
ggnmem version
ggnmem version --verbose
``````

## Uninstall

``````powershell
ggnmem uninstall          # keeps database
ggnmem uninstall --purge  # removes everything
``````

## Directory Layout

| Path | Purpose |
|------|---------|
| ``%LOCALAPPDATA%\ggnmem\bin\ggnmem.exe`` | CLI binary |
| ``%LOCALAPPDATA%\ggnmem\bin\ggnmem-daemon.exe`` | Background daemon |
| ``%APPDATA%\ggnmem\config.toml`` | Configuration |
| ``%LOCALAPPDATA%\ggnmem\data\ggnmem.db`` | Command database |
| ``%LOCALAPPDATA%\ggnmem\models\`` | AI embedding models |
| ``%LOCALAPPDATA%\ggnmem\logs\`` | Runtime logs |

## License

MIT OR Apache-2.0
"@ | Set-Content (Join-Path $ReleaseDir "README.md") -NoNewline
Write-Host "  README.md"

# Generate checksums inside the release directory.
Write-Host "Generating checksums..."
$checksumLines = @()
foreach ($file in @("ggnmem.exe", "ggnmem-daemon.exe", "VERSION", "README.md")) {
    $filePath = Join-Path $ReleaseDir $file
    $hash = (Get-FileHash -Path $filePath -Algorithm SHA256).Hash.ToLower()
    $checksumLines += "$hash  $file"
}
$checksumLines -join "`n" | Set-Content (Join-Path $ReleaseDir "checksums.txt") -NoNewline
Write-Host "  checksums.txt"

# ─── Create ZIP ──────────────────────────────────────────────────────────────

$ZipName = "ggnmem-windows-${Arch}.zip"
$ZipPath = Join-Path $ProjectRoot $ZipName

Write-Host "Creating $ZipName..."

# Remove existing ZIP if present.
if (Test-Path $ZipPath) {
    Remove-Item $ZipPath -Force
}

Compress-Archive -Path (Join-Path $ReleaseDir "*") -DestinationPath $ZipPath -CompressionLevel Optimal

# ─── Verify ZIP integrity ────────────────────────────────────────────────────

Write-Host "Verifying ZIP integrity..."

$VerifyDir = Join-Path $env:TEMP "ggnmem-verify-$(Get-Random)"
New-Item -ItemType Directory -Path $VerifyDir -Force | Out-Null

Expand-Archive -Path $ZipPath -DestinationPath $VerifyDir -Force

$requiredFiles = @("ggnmem.exe", "ggnmem-daemon.exe", "VERSION", "README.md", "checksums.txt")
$missing = @()
foreach ($file in $requiredFiles) {
    if (-not (Test-Path (Join-Path $VerifyDir $file))) {
        $missing += $file
    }
}

if ($missing.Count -gt 0) {
    Write-Error "ERROR: ZIP is missing required files: $($missing -join ', ')"
    Remove-Item $VerifyDir -Recurse -Force
    exit 1
}

# Verify checksums inside the extracted ZIP.
$checksumContent = Get-Content (Join-Path $VerifyDir "checksums.txt")
foreach ($line in $checksumContent) {
    if ([string]::IsNullOrWhiteSpace($line)) { continue }
    $parts = $line -split '\s+'
    $expectedHash = $parts[0]
    $fileName = $parts[1]
    $filePath = Join-Path $VerifyDir $fileName
    if (Test-Path $filePath) {
        $actualHash = (Get-FileHash -Path $filePath -Algorithm SHA256).Hash.ToLower()
        if ($expectedHash -ne $actualHash) {
            Write-Error "ERROR: Checksum mismatch for $fileName"
            Remove-Item $VerifyDir -Recurse -Force
            exit 1
        }
    }
}

Write-Host "ZIP checksums verified."

# Clean up.
Remove-Item $VerifyDir -Recurse -Force
Remove-Item $ReleaseDir -Recurse -Force

# ─── Summary ─────────────────────────────────────────────────────────────────

$ZipSize = "{0:N2} MB" -f ((Get-Item $ZipPath).Length / 1MB)

Write-Host ""
Write-Host "==================================="
Write-Host "  Release assembled: $ZipName"
Write-Host "==================================="
Write-Host "  Version:   $Version"
Write-Host "  Commit:    $Commit"
Write-Host "  Arch:      $Arch"
Write-Host "  Platform:  windows"
Write-Host "  ZIP size:  $ZipSize"
Write-Host ""
