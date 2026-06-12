# ggnmem v__VERSION__

## What's New

<!-- Add changelog entries here before publishing -->

- Feature: ...
- Fix: ...
- Improvement: ...

## Installation

### Quick Install (from tarball)

```bash
tar xzf ggnmem-linux-__ARCH__.tar.gz
bash install.sh
```

### Upgrade Existing Installation

```bash
ggnmem upgrade --bundle ggnmem-linux-__ARCH__.tar.gz
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
