# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.4.x (current) | ✅ |
| < 0.4.0 | ❌ |

## Reporting a Vulnerability

If you discover a security vulnerability in ggnmem, **please report it privately**.

**Do NOT open a public GitHub Issue for security vulnerabilities.**

### How to Report

Send a detailed report to:

📧 **gaganrajput1008@gmail.com**

Please include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### Response Timeline

- **Acknowledgment**: Within 48 hours
- **Assessment**: Within 1 week
- **Fix or mitigation**: As soon as practical

### What Happens Next

1. We will confirm receipt of your report.
2. We will investigate and assess the severity.
3. We will develop and test a fix.
4. We will release a patch and credit you (unless you prefer anonymity).

## Security Design

ggnmem is built with a **local-first, privacy-first** architecture:

- **No network requests** in normal operation (after initial setup)
- **No telemetry or analytics**
- **No cloud storage** of any kind
- **No authentication tokens** stored by the application
- All data remains on the user's machine
- Database files use user-restricted permissions
- IPC sockets are restricted to the local user

### Secret Redaction

ggnmem automatically redacts commands that appear to contain sensitive data
before storing them. The filter (`ggnmem-db/src/filter.rs`) detects patterns
such as:
- `password=`
- `token=`
- `api_key=` / `apikey=`
- `secret_key=`
- `--auth` with inline credentials
- Environment variable assignments with sensitive names

### Contributor Security Requirements

Contributors must **never** include in any commit:
- API keys, passwords, tokens, or private keys
- Real user command history
- Personal data or real IP addresses
- Machine-specific configuration
- `.env` files

See [CONTRIBUTING.md](CONTRIBUTING.md) for full guidelines.
