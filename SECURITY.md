# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.x.x   | ✅ Active support |

## Reporting a Vulnerability

If you discover a security vulnerability in ClipSave, please report it responsibly:

1. **Do NOT** open a public GitHub issue for security vulnerabilities
2. Email the maintainers at [security contact to be configured]
3. Include a detailed description of the vulnerability
4. Include steps to reproduce if possible

We will acknowledge receipt within 48 hours and provide a timeline for a fix.

## Security Model

### Minimum Capabilities (Tauri v2)

ClipSave operates with the smallest possible set of system permissions:

- **Filesystem**: Scoped to user-selected download directory only
- **Dialog**: Directory selection dialog
- **Clipboard**: Explicit read/write (no background polling)
- **Opener**: Open files and folders within download directory
- **HTTP**: Outgoing requests to public hosts only

### What ClipSave Does NOT Do

- ❌ Execute arbitrary shell commands
- ❌ Access filesystem outside the download directory
- ❌ Store cookies, account tokens, or credentials
- ❌ Make requests with forged or deceptive headers
- ❌ Bypass platform access controls or DRM
- ❌ Poll clipboard in the background
- ❌ Send telemetry or user data to third parties

### Path Traversal Protection

All file paths from the frontend are validated to resolve within the configured download directory. Path traversal attempts (`..`, absolute paths outside base, symlink escapes) are rejected with a `PermissionDenied` error.

### Network Security

- HTTP request timeouts: 15s for metadata, 120s for downloads
- Maximum redirect hops: 5
- Non-HTTP(S) redirect targets are rejected
- Generic, non-deceptive User-Agent header
- No cookies or authentication tokens stored or sent

### Data Privacy

- No user credentials stored
- Structured logs redact query string values and sensitive data
- Error reports shown to users are redacted (no full URLs, paths, or tokens)
- Clipboard content is not persisted beyond the current parsing request
