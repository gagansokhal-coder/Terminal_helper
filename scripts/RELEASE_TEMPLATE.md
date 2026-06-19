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

```bash
curl -fsSL https://raw.githubusercontent.com/gagansokhal-coder/Terminal_helper/main/scripts/install-online.sh | bash
```

### Upgrade Existing Installation

```bash
ggnmem self-update
```

### Manual Install (from tarball)

```bash
tar xzf ggnmem-linux-__ARCH__.tar.gz
bash install.sh
```

### Verify Installation

```bash
ggnmem version
ggnmem doctor
```

## Downloads

| Asset | SHA256 |
|-------|--------|
| `ggnmem-linux-__ARCH__.tar.gz` | `__TARBALL_SHA256__` |

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

- Linux (x86_64 or aarch64) or WSL
- No Rust toolchain required (pre-built binaries)
- ~100 MB disk space (with AI model)

## Preserved During Upgrade

The following user data is automatically preserved during upgrades:

- `~/.config/ggnmem/config.toml` — configuration
- `~/.local/share/ggnmem/ggnmem.db` — command history database
- `~/.local/share/ggnmem/models/` — installed AI models

## Uninstall

```bash
ggnmem uninstall          # keeps database
ggnmem uninstall --full   # removes everything
```
