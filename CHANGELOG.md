# Changelog

All notable changes to Observer Lite are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Observer Lite uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added
- Initial public release of Observer Lite
- HTTP/HEAD/POST monitor types with flexible status code DSL (`2xx`, `!5xx`, `200|404`)
- Response body matching: `contains`, `equals`, `not_equals`
- Response time warning threshold per monitor
- Configurable retry before alerting (`retry_count` + `retry_interval_seconds`)
- Ping (ICMP) monitor type via system `ping` binary
- SSL certificate validity and expiry monitoring
- Heartbeat (push-based) monitor type with grace period
- Maintenance windows with optional cron recurrence
- APScheduler-backed job scheduler persisted to SQLite (survives container restarts)
- Incident state machine: open on N consecutive failures, close on recovery
- Recovery notifications with downtime duration
- Email notifications via async SMTP (aiosmtplib)
- Telegram notifications via Bot API
- Per-monitor notification channel assignment (`on_down`, `on_recovery`, `on_ssl_warn` flags)
- Multi-user system with roles: `superadmin`, `admin`, `viewer`
- Per-monitor user assignment with individual alert opt-in
- Monitor groups for organisational grouping
- API keys with prefix `obs_`, SHA-256 stored, optional expiry
- Audit log for all write operations
- Admin-configurable data retention (default 90 days), enforced by a nightly cleanup job
- Daily SSL expiry scan job
- Server-Sent Events (SSE) real-time dashboard updates
- JSON import/export for monitor configurations
- Stats API: uptime %, average response time, total incidents (configurable time window)
- 90-day uptime bar visualisation
- Response time chart (uPlot, ~40 KB)
- Dark-mode Vue 3 frontend (Tailwind CSS, no CDN dependencies)
- Single Docker image: multi-stage build, embedded SQLite, `VOLUME ["/data"]`
- Forced password change for the default `admin` account on first login
- `slowapi` rate limiting on login (5/min/IP) and heartbeat (1/10 s/token)
- SMTP and Telegram credentials encrypted at rest (Fernet / AES-128-CBC)
- SECURITY.md, CONTRIBUTING.md, and this CHANGELOG

---

## [0.1.0] — 2026-05-05

Initial release. See [Unreleased] for the full feature list — all items above ship in 0.1.0.

[Unreleased]: https://github.com/cagatayuresin/observer-lite/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/cagatayuresin/observer-lite/releases/tag/v0.1.0
