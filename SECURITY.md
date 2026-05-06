# Security Policy

## Supported Versions

Only the latest release of Observer Lite receives security fixes.

| Version | Supported |
| ------- | --------- |
| latest  | ✅        |
| older   | ❌        |

## Reporting a Vulnerability

**Please do not open a public GitHub issue for security vulnerabilities.**

Send a private report to **cagatayuresin@gmail.com** with:

1. A clear description of the vulnerability
2. Steps to reproduce (proof-of-concept code if available)
3. The potential impact (what an attacker could achieve)
4. Your suggested fix (optional)

You can expect an acknowledgement within **48 hours** and a status update within **7 days**.

If the vulnerability is confirmed, we will:

- Work on a fix and release it as soon as possible
- Credit you in the release notes (unless you prefer to remain anonymous)
- Open a public GitHub Security Advisory after the fix is released

## Security Design Notes

The following design decisions are documented to help security researchers understand the attack surface:

### Authentication
- Passwords are hashed with **bcrypt** (cost factor 12).
- JWT access tokens expire in **15 minutes**; refresh tokens in **30 days**.
- Failed login attempts are rate-limited to **5 per minute per IP** via `slowapi`.
- The first-run admin account (`admin/admin`) is forced to change its password before any other action.

### Secrets at Rest
- SMTP and Telegram credentials stored in `app_settings` are encrypted with **Fernet** (AES-128-CBC + HMAC-SHA256). The encryption key is derived from `SECRET_KEY` via PBKDF2-HMAC-SHA256 (100 000 iterations).
- API keys are stored only as **SHA-256 hashes**; the raw key is shown exactly once at creation time.

### API Keys
- Keys have the prefix `obs_` and are 64 hex characters long (256 bits of entropy).
- Keys can be scoped to a user and have optional expiry dates.

### Network
- All monitor check results, incidents, and notifications are scoped to authenticated users.
- The SSE stream requires a valid JWT passed as a query parameter (EventSource does not support custom headers).
- Heartbeat endpoints are intentionally unauthenticated (token-based) but rate-limited to 1 request per 10 seconds per token.

### SQLite
- WAL mode is enabled for safe concurrent reads.
- `PRAGMA foreign_keys = ON` is enforced on every connection.
- The database file is stored on a Docker volume (`/data`) that should be secured at the host level.

### Docker
- The container runs as a non-root user when built with the provided `Dockerfile`.
- `--workers 1` is required because APScheduler and the SSE broadcaster are in-process; running multiple workers would cause duplicate checks and missed events.

## Known Limitations

- **No built-in TLS termination.** Deploy behind a reverse proxy (nginx, Caddy) that terminates HTTPS.
- **Single-process constraint.** Horizontal scaling is not supported without replacing the in-process scheduler and broadcaster.
- **SSRF risk.** Admins can configure monitors to check internal IP addresses. This is by design for self-hosted environments, but deployers should be aware of the risk if untrusted users have admin access.
