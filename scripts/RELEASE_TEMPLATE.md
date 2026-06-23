# ggnmem v__VERSION__

## 🌐 Website

**[ggnmem.mytechy.in](https://ggnmem.mytechy.in)**

## What's New

<!-- Add changelog entries here before publishing -->

- Feature: ...
- Fix: ...
- Improvement: ...

## Installation

### One-Line Install

**Linux / WSL:**
```bash
curl -fsSL https://raw.githubusercontent.com/gagansokhal-coder/Terminal_helper/main/scripts/install-online.sh | bash
```

**Windows:**
```powershell
irm https://ggnmem.mytechy.in/install.ps1 | iex
```

### Upgrade Existing Installation

```bash
ggnmem self-update
```

### Manual Install (from bundle)

**Linux:**
```bash
tar xzf ggnmem-linux-__ARCH__.tar.gz
bash install.sh
```

**Windows:**
Extract `ggnmem-windows-x86_64.zip` and run `install.ps1`.

### Verify Installation

```bash
ggnmem version
ggnmem doctor
```

## Downloads

| Asset | SHA256 |
|-------|--------|
| `ggnmem-linux-x86_64.tar.gz` | `__TARBALL_SHA256__` |
| `ggnmem-linux-aarch64.tar.gz` | `__TARBALL_AARCH64_SHA256__` |
| `ggnmem-windows-x86_64.zip` | `__ZIP_SHA256__` |

## Build Info

| Field | Value |
|-------|-------|
| Version | __VERSION__ |
| Commit | __COMMIT__ |
| Date | __DATE__ |
| Rust | __RUSTC_VERSION__ |
| Platform | linux-__ARCH__ |
| ONNX | enabled |

## Requirements

- Linux (x86_64 or aarch64) or WSL or Windows (x86_64)
- No Rust toolchain required (pre-built binaries)
- ~100 MB disk space (with AI model)

## Preserved During Upgrade

The following user data is automatically preserved during upgrades:

- `~/.config/ggnmem/` or `%APPDATA%\ggnmem\` — configuration
- `~/.local/share/ggnmem/` or `%LOCALAPPDATA%\ggnmem\data\` — command history database
- `~/.local/share/ggnmem/models/` or `%LOCALAPPDATA%\ggnmem\models\` — installed AI models

## Uninstall

```bash
ggnmem uninstall          # keeps database
ggnmem uninstall --full   # removes everything
```
