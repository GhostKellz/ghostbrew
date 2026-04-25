# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.3.x   | :white_check_mark: |
| < 0.3   | :x:                |

## Reporting a Vulnerability

If you discover a security vulnerability in GhostBrew, please report it responsibly:

1. **Email**: Send details to ckelley@ghostkellz.sh
2. **GitHub Security Advisories**: Use [GitHub's private vulnerability reporting](https://github.com/ghostkellz/ghostbrew/security/advisories/new)

Please include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

## Response Timeline

- **Acknowledgment**: Within 48 hours
- **Initial assessment**: Within 7 days
- **Fix timeline**: Depends on severity, typically within 30 days for critical issues

## Security Considerations

GhostBrew is a BPF scheduler that runs with elevated privileges. Security considerations include:

### BPF Code
- All BPF code is verified by the kernel's BPF verifier before loading
- BPF programs run in a sandboxed environment with limited kernel access
- No network or filesystem access from BPF context

### Userspace Daemon
- Requires root or CAP_BPF/CAP_SYS_ADMIN capabilities to load the scheduler
- Control interface at `/run/ghostbrew/control` is root-only (mode 0600)
- Configuration files are read-only after initial load

### Runtime Security
- Scheduler gracefully falls back to EEVDF on exit or crash
- No persistent kernel modifications
- BPF maps are cleaned up on scheduler exit

## Scope

The following are **in scope** for security reports:
- Privilege escalation vulnerabilities
- BPF verifier bypasses
- Denial of service affecting system stability
- Information disclosure from BPF maps

The following are **out of scope**:
- Issues requiring root access (scheduler requires root by design)
- Performance degradation (not a security issue)
- Issues in upstream dependencies (report to upstream maintainers)
