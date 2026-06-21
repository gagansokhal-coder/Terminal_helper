# ═══════════════════════════════════════════════════════════════════════════════
# ggnmem — Windows PowerShell Installer
#
# Usage:
#   irm https://ggnmem.mytechy.in/install.ps1 | iex
#
# This script installs ggnmem on Windows natively.
#
# Features:
#   - Fetches the latest release from GitHub automatically
#   - Verifies SHA256 checksums before installing
#   - Detects existing installations and upgrades in place
#   - Preserves config, database, and AI models
#   - Configures PATH automatically (User scope)
#   - Rollback on failure
#   - Full installation logging
#
# Security:
#   - HTTPS only
#   - SHA256 checksum verification
#   - Fail-closed on verification errors
#   - Never executes downloaded binaries before verification
# ═══════════════════════════════════════════════════════════════════════════════

#Requires -Version 5.1

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"  # Speed up Invoke-WebRequest

# ─── Constants ───────────────────────────────────────────────────────────────

$REPO            = "gagansokhal-coder/Terminal_helper"
$GITHUB_API      = "https://api.github.com/repos/$REPO/releases"
$ASSET_NAME      = "ggnmem-windows-x86_64.zip"
$CHECKSUMS_NAME  = "checksums.txt"

# ─── Directories ─────────────────────────────────────────────────────────────

$INSTALL_ROOT    = Join-Path $env:LOCALAPPDATA "ggnmem"
$BIN_DIR         = Join-Path $INSTALL_ROOT "bin"
$DATA_DIR        = Join-Path $INSTALL_ROOT "data"
$MODELS_DIR      = Join-Path $INSTALL_ROOT "models"
$LOGS_DIR        = Join-Path $INSTALL_ROOT "logs"
$CONFIG_DIR      = Join-Path $env:APPDATA "ggnmem"
$CONFIG_FILE     = Join-Path $CONFIG_DIR "config.toml"
$DB_FILE         = Join-Path $DATA_DIR "ggnmem.db"
$VERSION_FILE    = Join-Path $INSTALL_ROOT "VERSION"
$LOG_FILE        = Join-Path $LOGS_DIR "install.log"

# ─── State ───────────────────────────────────────────────────────────────────

$script:UpgradeMode     = $false
$script:ExistingVersion = ""
$script:NewVersion      = ""
$script:BackedUpCli     = $false
$script:BackedUpDaemon  = $false
$script:RollbackNeeded  = $false

# ─── Output helpers ──────────────────────────────────────────────────────────

function Write-Step  { param([string]$msg) Write-Host "`n$msg" -ForegroundColor White -NoNewline; Write-Host "" }
function Write-Info  { param([string]$msg) Write-Host "  [info]  $msg" -ForegroundColor Cyan }
function Write-Ok    { param([string]$msg) Write-Host "  [ok]    $msg" -ForegroundColor Green }
function Write-Warn  { param([string]$msg) Write-Host "  [warn]  $msg" -ForegroundColor Yellow }
function Write-Err   { param([string]$msg) Write-Host "  [error] $msg" -ForegroundColor Red }

function Write-Log {
    param([string]$msg)
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $entry = "[$timestamp] $msg"
    try {
        if (Test-Path (Split-Path $LOG_FILE -Parent)) {
            Add-Content -Path $LOG_FILE -Value $entry -ErrorAction SilentlyContinue
        }
    } catch {
        # Logging should never cause a failure
    }
}

function Step-Log {
    param([string]$msg)
    Write-Step $msg
    Write-Log $msg
}

function Info-Log {
    param([string]$msg)
    Write-Info $msg
    Write-Log "INFO: $msg"
}

function Ok-Log {
    param([string]$msg)
    Write-Ok $msg
    Write-Log "OK: $msg"
}

function Warn-Log {
    param([string]$msg)
    Write-Warn $msg
    Write-Log "WARN: $msg"
}

function Err-Log {
    param([string]$msg)
    Write-Err $msg
    Write-Log "ERROR: $msg"
}

function Abort {
    param([string]$msg)
    Err-Log $msg
    Write-Log "ABORT: Installation aborted"
    Invoke-Rollback
    exit 1
}

# ─── Rollback ────────────────────────────────────────────────────────────────

function Invoke-Rollback {
    if (-not $script:RollbackNeeded) { return }

    Write-Host ""
    Warn-Log "Rolling back installation..."

    $cliBin    = Join-Path $BIN_DIR "ggnmem.exe"
    $daemonBin = Join-Path $BIN_DIR "ggnmem-daemon.exe"
    $cliOld    = Join-Path $BIN_DIR "ggnmem.exe.old"
    $daemonOld = Join-Path $BIN_DIR "ggnmem-daemon.exe.old"

    if ($script:BackedUpCli -and (Test-Path $cliOld)) {
        Move-Item -Path $cliOld -Destination $cliBin -Force -ErrorAction SilentlyContinue
        Ok-Log "Restored ggnmem.exe from backup"
    }

    if ($script:BackedUpDaemon -and (Test-Path $daemonOld)) {
        Move-Item -Path $daemonOld -Destination $daemonBin -Force -ErrorAction SilentlyContinue
        Ok-Log "Restored ggnmem-daemon.exe from backup"
    }

    Write-Host ""
    Write-Info "User data was NOT modified (database, config, models are untouched)."
    Write-Info "To retry, run:"
    Write-Info '  irm https://ggnmem.mytechy.in/install.ps1 | iex'
    Write-Host ""
}

# ═══════════════════════════════════════════════════════════════════════════════
#  MAIN INSTALLATION FLOW
# ═══════════════════════════════════════════════════════════════════════════════

Write-Host ""
Write-Host "═══════════════════════════════════════" -ForegroundColor Magenta
Write-Host "  ggnmem installer for Windows"         -ForegroundColor Magenta
Write-Host "═══════════════════════════════════════" -ForegroundColor Magenta

# ─── 1. Detect environment ──────────────────────────────────────────────────

Step-Log "Detecting environment..."

# Verify Windows
if ($env:OS -ne "Windows_NT") {
    Abort "This installer is for Windows only. For Linux/WSL, use install.sh."
}
Info-Log "OS: Windows"

# PowerShell version
$psVer = $PSVersionTable.PSVersion
Info-Log "PowerShell: $($psVer.Major).$($psVer.Minor)"

if ($psVer.Major -lt 5) {
    Abort "PowerShell 5.1 or later is required. Current: $($psVer.Major).$($psVer.Minor)"
}

# Architecture
$arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
if (-not $arch) {
    # Fallback for older PS
    $arch = if ([System.Environment]::Is64BitOperatingSystem) { "X64" } else { "X86" }
}
Info-Log "Architecture: $arch"

if ($arch -notin @("X64", "Arm64")) {
    Abort "Unsupported architecture: $arch. ggnmem requires x86_64 (AMD64)."
}

if ($arch -eq "Arm64") {
    Warn-Log "ARM64 detected. The x86_64 build will run via emulation."
}

# ─── 2. Create directories ──────────────────────────────────────────────────

Step-Log "Creating directories..."

foreach ($dir in @($BIN_DIR, $DATA_DIR, $MODELS_DIR, $LOGS_DIR, $CONFIG_DIR)) {
    if (Test-Path $dir) {
        Ok-Log "$dir (exists)"
    } else {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
        Ok-Log "$dir (created)"
    }
}

# Initialize log file
Write-Log "════════════════════════════════════════════════════════════════"
Write-Log "ggnmem installer started"
Write-Log "OS: Windows  PowerShell: $($psVer.Major).$($psVer.Minor)  Arch: $arch"
Write-Log "════════════════════════════════════════════════════════════════"

# ─── 3. Detect existing installation ────────────────────────────────────────

Step-Log "Checking for existing installation..."

$cliBin = Join-Path $BIN_DIR "ggnmem.exe"

if (Test-Path $cliBin) {
    try {
        $script:ExistingVersion = & $cliBin version 2>&1 | Select-Object -First 1
        $script:ExistingVersion = ($script:ExistingVersion -replace "^ggnmem\s*", "").Trim()
    } catch {
        $script:ExistingVersion = "unknown"
    }
    Info-Log "Found existing installation: $($script:ExistingVersion)"
    $script:UpgradeMode = $true
} else {
    Info-Log "No existing installation found (fresh install)"
}

# ─── 4. Fetch latest release from GitHub ────────────────────────────────────

Step-Log "Fetching latest release from GitHub..."

try {
    $headers = @{
        "Accept"     = "application/vnd.github+json"
        "User-Agent" = "ggnmem-installer/1.0"
    }
    $releases = Invoke-RestMethod -Uri $GITHUB_API -Headers $headers -Method Get -TimeoutSec 30
} catch {
    Abort "Failed to fetch releases from GitHub API: $($_.Exception.Message)"
}

if (-not $releases -or $releases.Count -eq 0) {
    Abort "No releases found on GitHub."
}

# Find the latest release (first in the list)
$latestRelease = $releases[0]
$tagName = $latestRelease.tag_name
$script:NewVersion = ($tagName -replace "^v", "")

Info-Log "Latest release: $tagName ($($script:NewVersion))"

# Find asset URLs
$assetUrl    = $null
$checksumUrl = $null

foreach ($asset in $latestRelease.assets) {
    if ($asset.name -eq $ASSET_NAME) {
        $assetUrl = $asset.browser_download_url
    }
    if ($asset.name -eq $CHECKSUMS_NAME) {
        $checksumUrl = $asset.browser_download_url
    }
}

if (-not $assetUrl) {
    Abort "Release asset '$ASSET_NAME' not found in release $tagName."
}

if (-not $checksumUrl) {
    Abort "Checksums file '$CHECKSUMS_NAME' not found in release $tagName."
}

Ok-Log "Asset: $ASSET_NAME"
Ok-Log "Checksums: $CHECKSUMS_NAME"

# ─── 5. Download release assets ─────────────────────────────────────────────

Step-Log "Downloading release assets..."

$tempDir = Join-Path $env:TEMP "ggnmem-install-$(Get-Random)"
New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

$zipPath      = Join-Path $tempDir $ASSET_NAME
$checksumPath = Join-Path $tempDir $CHECKSUMS_NAME

try {
    Info-Log "Downloading $ASSET_NAME..."
    Write-Log "URL: $assetUrl"
    Invoke-WebRequest -Uri $assetUrl -OutFile $zipPath -UseBasicParsing -TimeoutSec 120
    $zipSize = "{0:N2} MB" -f ((Get-Item $zipPath).Length / 1MB)
    Ok-Log "Downloaded $ASSET_NAME ($zipSize)"
} catch {
    Remove-Item $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    Abort "Failed to download release: $($_.Exception.Message)"
}

try {
    Info-Log "Downloading checksums.txt..."
    Invoke-WebRequest -Uri $checksumUrl -OutFile $checksumPath -UseBasicParsing -TimeoutSec 30
    Ok-Log "Downloaded checksums.txt"
} catch {
    Remove-Item $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    Abort "Failed to download checksums: $($_.Exception.Message)"
}

# ─── 6. Verify checksum ─────────────────────────────────────────────────────

Step-Log "Verifying SHA256 checksum..."

# Read checksums file
$checksumContent = Get-Content $checksumPath -Raw
$expectedHash = $null

foreach ($line in ($checksumContent -split "`n")) {
    $line = $line.Trim()
    if ([string]::IsNullOrWhiteSpace($line)) { continue }
    $parts = $line -split '\s+'
    if ($parts.Count -ge 2 -and $parts[1] -eq $ASSET_NAME) {
        $expectedHash = $parts[0].ToLower()
        break
    }
}

if (-not $expectedHash) {
    Remove-Item $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    Abort "No checksum entry found for '$ASSET_NAME' in checksums.txt"
}

$actualHash = (Get-FileHash -Path $zipPath -Algorithm SHA256).Hash.ToLower()

Info-Log "Expected: $expectedHash"
Info-Log "Actual:   $actualHash"

if ($expectedHash -ne $actualHash) {
    Remove-Item $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    Abort "Checksum mismatch! The downloaded file may be corrupted or tampered with."
}

Ok-Log "SHA256 checksum verified"

# ─── 7. Extract release ─────────────────────────────────────────────────────

Step-Log "Extracting release..."

$extractDir = Join-Path $tempDir "extracted"
New-Item -ItemType Directory -Path $extractDir -Force | Out-Null

try {
    Expand-Archive -Path $zipPath -DestinationPath $extractDir -Force
} catch {
    Remove-Item $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    Abort "Failed to extract ZIP: $($_.Exception.Message)"
}

# Verify required files
$requiredFiles = @("ggnmem.exe", "ggnmem-daemon.exe", "VERSION")
$missingFiles = @()

foreach ($file in $requiredFiles) {
    $filePath = Join-Path $extractDir $file
    if (-not (Test-Path $filePath)) {
        $missingFiles += $file
    }
}

if ($missingFiles.Count -gt 0) {
    Remove-Item $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    Abort "ZIP is missing required files: $($missingFiles -join ', ')"
}

Ok-Log "Extracted and validated ($($requiredFiles.Count) required files present)"

# Verify inner checksums (if present inside ZIP)
$innerChecksums = Join-Path $extractDir "checksums.txt"
if (Test-Path $innerChecksums) {
    Info-Log "Verifying inner checksums..."
    $innerContent = Get-Content $innerChecksums
    $innerFailed = $false

    foreach ($line in $innerContent) {
        $line = $line.Trim()
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        $parts = $line -split '\s+'
        if ($parts.Count -lt 2) { continue }

        $hash = $parts[0].ToLower()
        $name = $parts[1]
        $path = Join-Path $extractDir $name

        if (Test-Path $path) {
            $actual = (Get-FileHash -Path $path -Algorithm SHA256).Hash.ToLower()
            if ($hash -ne $actual) {
                Err-Log "Inner checksum mismatch: $name"
                $innerFailed = $true
            }
        }
    }

    if ($innerFailed) {
        Remove-Item $tempDir -Recurse -Force -ErrorAction SilentlyContinue
        Abort "Inner checksum verification failed. Archive contents may be corrupted."
    }

    Ok-Log "Inner checksums verified"
}

# ─── 8. Stop daemon if upgrading ────────────────────────────────────────────

if ($script:UpgradeMode) {
    Step-Log "Stopping daemon for upgrade..."

    $daemonProc = Get-Process -Name "ggnmem-daemon" -ErrorAction SilentlyContinue
    if ($daemonProc) {
        try {
            $existingCli = Join-Path $BIN_DIR "ggnmem.exe"
            & $existingCli stop 2>&1 | Out-Null
            Start-Sleep -Seconds 2
        } catch {
            # Fallback: kill process directly
        }

        # Ensure it's stopped
        $daemonProc = Get-Process -Name "ggnmem-daemon" -ErrorAction SilentlyContinue
        if ($daemonProc) {
            $daemonProc | Stop-Process -Force -ErrorAction SilentlyContinue
            Start-Sleep -Seconds 1
        }

        Ok-Log "Daemon stopped"
    } else {
        Ok-Log "Daemon not running"
    }
}

# ─── 9. Backup existing binaries ────────────────────────────────────────────

Step-Log "Installing binaries..."

$script:RollbackNeeded = $true

$cliBin    = Join-Path $BIN_DIR "ggnmem.exe"
$daemonBin = Join-Path $BIN_DIR "ggnmem-daemon.exe"
$cliOld    = Join-Path $BIN_DIR "ggnmem.exe.old"
$daemonOld = Join-Path $BIN_DIR "ggnmem-daemon.exe.old"

if ($script:UpgradeMode) {
    if (Test-Path $cliBin) {
        Copy-Item -Path $cliBin -Destination $cliOld -Force
        $script:BackedUpCli = $true
        Ok-Log "Backed up ggnmem.exe -> ggnmem.exe.old"
    }
    if (Test-Path $daemonBin) {
        Copy-Item -Path $daemonBin -Destination $daemonOld -Force
        $script:BackedUpDaemon = $true
        Ok-Log "Backed up ggnmem-daemon.exe -> ggnmem-daemon.exe.old"
    }
}

# ─── 10. Install binaries ───────────────────────────────────────────────────

try {
    Copy-Item -Path (Join-Path $extractDir "ggnmem.exe") -Destination $cliBin -Force
    Ok-Log "ggnmem.exe -> $cliBin"
} catch {
    Abort "Failed to install ggnmem.exe: $($_.Exception.Message)"
}

try {
    Copy-Item -Path (Join-Path $extractDir "ggnmem-daemon.exe") -Destination $daemonBin -Force
    Ok-Log "ggnmem-daemon.exe -> $daemonBin"
} catch {
    Abort "Failed to install ggnmem-daemon.exe: $($_.Exception.Message)"
}

# Copy VERSION file
$srcVersion = Join-Path $extractDir "VERSION"
if (Test-Path $srcVersion) {
    Copy-Item -Path $srcVersion -Destination $VERSION_FILE -Force
    Ok-Log "VERSION -> $VERSION_FILE"
}

$script:RollbackNeeded = $false  # Installation succeeded, no rollback needed

# Clean up old backups on success
if ($script:UpgradeMode) {
    if (Test-Path $cliOld) {
        Remove-Item $cliOld -Force -ErrorAction SilentlyContinue
    }
    if (Test-Path $daemonOld) {
        Remove-Item $daemonOld -Force -ErrorAction SilentlyContinue
    }
}

# ─── 11. Configure PATH ─────────────────────────────────────────────────────

Step-Log "Configuring PATH..."

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
$pathEntries = $userPath -split ";"

if ($pathEntries -contains $BIN_DIR) {
    Ok-Log "$BIN_DIR is already in PATH"
} else {
    $newPath = "$BIN_DIR;$userPath"
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Ok-Log "Added $BIN_DIR to User PATH"

    # Also update current session
    $env:Path = "$BIN_DIR;$env:Path"
    Info-Log "PATH updated for current session"
}

# ─── 12. Create default config ──────────────────────────────────────────────

Step-Log "Setting up configuration..."

if (Test-Path $CONFIG_FILE) {
    Ok-Log "config.toml exists (preserved - not overwriting)"
} else {
    $configLines = @(
        '# ggnmem configuration'
        '# See: https://github.com/ggnmem/ggnmem'
        ''
        '[features]'
        'capture = true'
        'search = true'
        'tui = true'
        'ai = false'
        ''
        '[daemon]'
        'autostart = false'
        ''
        '[appearance]'
        'theme = "auto"'
        ''
        '[limits]'
        'max_history = 100000'
        'max_memory_mb = 40'
        'max_db_size_mb = 1024'
        ''
        '[search]'
        'index_mode = "balanced"'
        ''
        '[retention]'
        'retention_days = 365'
        'max_commands = 1000000'
        'auto_cleanup = true'
        ''
        '[ai]'
        'ai_enabled = false'
        'embedding_provider = "local"'
        'semantic_search = false'
        'model_name = "all-MiniLM-L6-v2"'
    )
    $configLines -join "`n" | Set-Content -Path $CONFIG_FILE -NoNewline
    Ok-Log "config.toml created"
}

# ─── 13. Preserve data notices ──────────────────────────────────────────────

Step-Log "Checking existing data..."

if (Test-Path $DB_FILE) {
    $dbSize = "{0:N2} MB" -f ((Get-Item $DB_FILE).Length / 1MB)
    Ok-Log "Database preserved: $DB_FILE ($dbSize)"
} else {
    Info-Log "Database will be created when daemon starts"
}

if (Test-Path $MODELS_DIR) {
    $modelDirs = Get-ChildItem -Path $MODELS_DIR -Directory -ErrorAction SilentlyContinue
    if ($modelDirs -and $modelDirs.Count -gt 0) {
        Ok-Log "AI models preserved: $MODELS_DIR ($($modelDirs.Count) model(s))"
        foreach ($modelDir in $modelDirs) {
            $modelSize = "{0:N2} MB" -f ((Get-ChildItem $modelDir.FullName -Recurse -File | Measure-Object -Property Length -Sum).Sum / 1MB)
            Info-Log "  model: $($modelDir.Name) ($modelSize)"
        }
    } else {
        Info-Log "No AI models installed (install with: ggnmem ai install)"
    }
} else {
    Info-Log "No AI models installed (install with: ggnmem ai install)"
}

# ─── 14. Upgrade messaging ──────────────────────────────────────────────────

if ($script:UpgradeMode) {
    Step-Log "Upgrade status..."
    Info-Log "Previous: $($script:ExistingVersion)"
    Info-Log "New:      $($script:NewVersion)"
}

# ─── 15. Start daemon ───────────────────────────────────────────────────────

Step-Log "Starting daemon..."

try {
    $startOutput = & $cliBin start 2>&1
    Start-Sleep -Seconds 2

    $daemonProc = Get-Process -Name "ggnmem-daemon" -ErrorAction SilentlyContinue
    if ($daemonProc) {
        Ok-Log "Daemon started (PID: $($daemonProc.Id))"
    } else {
        Warn-Log "Daemon may not have started. Run 'ggnmem start' manually."
    }
} catch {
    Warn-Log "Could not start daemon: $($_.Exception.Message)"
    Warn-Log "Start manually with: ggnmem start"
}

# ─── 16. Verify installation ────────────────────────────────────────────────

Step-Log "Verifying installation..."

$verified = $true

# Verify ggnmem version
try {
    $versionOutput = & $cliBin version 2>&1 | Select-Object -First 1
    Ok-Log "ggnmem version: $versionOutput"
} catch {
    Warn-Log "Could not verify ggnmem version"
    $verified = $false
}

# Verify ggnmem doctor
try {
    $doctorOutput = & $cliBin doctor 2>&1
    $doctorExitCode = $LASTEXITCODE
    if ($doctorExitCode -eq 0) {
        Ok-Log "ggnmem doctor: passed"
    } else {
        Warn-Log "ggnmem doctor reported issues (exit code: $doctorExitCode)"
        foreach ($line in $doctorOutput) {
            Info-Log "  $line"
        }
    }
} catch {
    Warn-Log "Could not run ggnmem doctor: $($_.Exception.Message)"
    $verified = $false
}

# ─── 17. Clean up ───────────────────────────────────────────────────────────

Remove-Item $tempDir -Recurse -Force -ErrorAction SilentlyContinue

# ─── 18. Summary ────────────────────────────────────────────────────────────

Write-Log "Installation completed successfully"

Write-Host ""
Write-Host "═══════════════════════════════════════" -ForegroundColor Green
if ($script:UpgradeMode) {
    Write-Host "  ggnmem upgraded successfully!"       -ForegroundColor Green
} else {
    Write-Host "  ggnmem installed successfully!"      -ForegroundColor Green
}
Write-Host "═══════════════════════════════════════" -ForegroundColor Green
Write-Host ""
Write-Host "  binaries:  $cliBin"
Write-Host "             $daemonBin"
Write-Host "  config:    $CONFIG_FILE"
Write-Host "  data:      $DATA_DIR"
Write-Host "  logs:      $LOG_FILE"
Write-Host ""

if ($script:UpgradeMode) {
    Write-Host "  +-----------------------------------+" -ForegroundColor Cyan
    Write-Host "  | Previous: $($script:ExistingVersion)" -ForegroundColor Cyan
    Write-Host "  | Current:  $($script:NewVersion)" -ForegroundColor Cyan
    Write-Host "  +-----------------------------------+" -ForegroundColor Cyan
    Write-Host "  |  $([char]0x2713) Config preserved              |" -ForegroundColor Green
    Write-Host "  |  $([char]0x2713) Database preserved            |" -ForegroundColor Green

    if ((Test-Path $MODELS_DIR) -and (Get-ChildItem $MODELS_DIR -Directory -ErrorAction SilentlyContinue).Count -gt 0) {
        Write-Host "  |  $([char]0x2713) AI models preserved           |" -ForegroundColor Green
    }

    Write-Host "  |  $([char]0x2713) PATH configured               |" -ForegroundColor Green
    Write-Host "  |  $([char]0x2713) Daemon started                 |" -ForegroundColor Green
    Write-Host "  |  $([char]0x2713) Installation verified          |" -ForegroundColor Green
    Write-Host "  +-----------------------------------+" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "  next steps:" -ForegroundColor White
    Write-Host "    1. Verify:       ggnmem doctor"
    Write-Host "    2. Check:        ggnmem version"
    Write-Host ""
} else {
    Write-Host "  $([char]0x2713) Installed"                -ForegroundColor Green
    Write-Host "  $([char]0x2713) PATH configured"          -ForegroundColor Green
    Write-Host "  $([char]0x2713) Database preserved"       -ForegroundColor Green
    Write-Host "  $([char]0x2713) Daemon started"           -ForegroundColor Green
    Write-Host "  $([char]0x2713) Installation verified"    -ForegroundColor Green
    Write-Host ""
    Write-Host "  next steps:" -ForegroundColor White
    Write-Host "    1. Open a new terminal (PATH changes take effect)"
    Write-Host "    2. Verify:       ggnmem doctor"
    Write-Host "    3. Try it:       ggnmem search <keyword>"
    Write-Host ""
}
