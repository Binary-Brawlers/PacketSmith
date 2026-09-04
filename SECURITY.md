# Security Policy

## Reporting Security Vulnerabilities

The PacketSmith team takes security seriously. If you believe you have discovered a vulnerability, please report it promptly so we can investigate and address it before public disclosure.

Please report security issues via email to: **security@packetsmith.dev** (or open a confidential security advisory on GitHub).

When reporting, please include:
- A description of the issue and potential impact
- Affected version(s) or commit hash
- Step-by-step instructions or proof-of-concept to reproduce the behavior
- Any recommended mitigation

We will acknowledge receipt within 48 hours and work with you to coordinate a responsible disclosure schedule.

---

## Security Invariants

PacketSmith enforces several core security invariants:

1. **Secret Containment:** Credentials, tokens, and private keys stored in the vault or environments are never written to unencrypted project files, git history, or application logs.
2. **Export Redaction:** Workspace and collection exports redact secrets by default unless the user explicitly commands a full vault export with an encryption passphrase.
3. **Safe Script Sandboxing:** Scripts (pre-request and test scripts) execute within isolated runtimes without uncontrolled access to the host file system or network outside authorized execution channels.
4. **TLS Integrity:** Certificate verification is enabled by default. Any insecure mode (disabling TLS verification) requires deliberate user confirmation and is clearly marked in the UI.
