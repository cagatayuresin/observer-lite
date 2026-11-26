<p align="center">
  <img src="frontend/public/logo.svg" width="96" height="96" alt="Observer Lite" />
</p>

<h1 align="center">Observer Lite</h1>

<p align="center">
  Lightweight, self-hosted uptime monitoring with email &amp; Telegram alerts,<br/>
  SSL certificate tracking, real-time dashboard, and a single Docker image.
</p>

<p align="center">
  <a href="https://github.com/cagatayuresin/observer-lite/tree/0.3.0-alpha">
    <img src="https://img.shields.io/badge/version-0.3.0--alpha.1-blue?style=flat-square" alt="Version 0.3.0-alpha.1" />
  </a>
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/license-MIT-green?style=flat-square" alt="MIT License" />
  </a>
  <img src="https://img.shields.io/badge/coverage-80%25-brightgreen?style=flat-square" alt="Test Coverage 80%" />
  <img src="https://img.shields.io/badge/rust-stable-000000?style=flat-square&logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/vue-3-42b883?style=flat-square&logo=vue.js&logoColor=white" alt="Vue 3" />
  <a href="https://github.com/users/cagatayuresin/packages/container/package/observer-lite">
    <img src="https://img.shields.io/badge/docker-ghcr.io-2496ed?style=flat-square&logo=docker&logoColor=white" alt="Docker Image on GHCR" />
  </a>
</p>

---

## Overview

**Observer Lite** is a self-hosted uptime monitoring tool built for simplicity and efficiency. It monitors up to 500+ endpoints simultaneously, tracks SSL certificate expiry, sends instant alerts via email and Telegram, and delivers a real-time dark-mode dashboard — all from a single Docker image with no external database required.

Think of it as a lighter, more flexible alternative to Uptime Kuma — with a built-in status code DSL, response body matching, API keys, audit logging, and configurable data retention.

Full documentation is available in [`docs/`](docs/) and can be published with GitHub Pages at:

```text
https://cagatayuresin.github.io/observer-lite/
```

---

## Features

| Category | Details |
|---|---|
| **Monitor Types** | HTTP GET / POST / HEAD, Ping (ICMP), SSL Certificate, Heartbeat |
| **Status Code Matching** | Flexible DSL: `2xx`, `!5xx`, `200\|301`, `2xx\|!503` |
| **Body Matching** | `contains`, `equals`, `not_equals` against response body |
| **SSL Monitoring** | Certificate validity + configurable expiry warning (default 30 days) |
| **Alerting** | Email (SMTP) and Telegram with cooldown, retry logic, and recovery notifications |
| **Incidents** | Automatic open/close lifecycle, acknowledgement, root cause tracking |
| **Maintenance Windows** | Suppress alerts during planned downtime, optional cron repeat |
| **Real-time Dashboard** | Server-Sent Events (SSE) push — no polling required |
| **User Management** | Roles: `superadmin`, `admin`, `viewer`; viewers see only monitors they created or are assigned to; forced password change on first login |
| **API Keys** | `obs_`-prefixed keys, SHA-256 hashed storage, per-key last-used tracking |
| **Audit Log** | Immutable record of all user actions |
| **Import / Export** | JSON-based monitor configuration portability |
| **Data Retention** | Configurable (default 90 days), auto-cleanup runs nightly |
| **UI** | Dark mode by default, fully bundled — zero CDN dependencies |

---

## Quick Start

### Docker Run

```bash
docker run -d \
  --name observer-lite \
  --restart unless-stopped \
  -p 3000:3000 \
  -v observer_data:/data \
  -e SECRET_KEY="$(openssl rand -hex 32)" \
  -e DATABASE_PATH="/data/observer.db" \
  ghcr.io/cagatayuresin/observer-lite:latest
```

Open [http://localhost:3000](http://localhost:3000) — log in with `admin` / `admin` and set a new password when prompted.

---

### Docker Compose

```yaml
services:
  observer-lite:
    image: ghcr.io/cagatayuresin/observer-lite:latest
    container_name: observer-lite
    restart: unless-stopped
    ports:
      - "3000:3000"
    volumes:
      - observer_data:/data
    environment:
      SECRET_KEY: "change-this-to-a-long-random-secret-key"
      DATABASE_PATH: "/data/observer.db"
      PORT: "3000"

volumes:
  observer_data:
```

```bash
docker compose up -d
```

---

## Configuration

All configuration is provided through environment variables.

| Variable | Required | Default | Description |
|---|---|---|---|
| `SECRET_KEY` | **Yes** | `change-me-…` | HMAC secret for JWT signing and credential encryption. Use a random 32+ byte value. |
| `DATABASE_PATH` | No | `/data/observer.db` | Path to the SQLite database file. |
| `PORT` | No | `3000` | Port the application listens on. |
| `DEBUG` | No | `false` | Enable debug logging (`true` / `false`). |

> **Security:** Never use the default `SECRET_KEY` in production. Generate one with `openssl rand -hex 32`.

---

## First Login

1. Navigate to `http://your-host:3000`
2. Log in with username **`admin`** and password **`admin`**
3. You will be immediately redirected to the password change screen
4. Set a strong password — the account is then ready to use

---

## Monitor Types

### HTTP (GET / POST / HEAD)
Sends an outbound HTTP request and evaluates the response against your rules.

- **Status Code DSL** — combine patterns with `|` (OR) and prefix `!` to negate:

  | Expression | Matches |
  |---|---|
  | `2xx` | Any 200–299 response |
  | `200` | Exactly 200 |
  | `2xx\|301` | Any 2xx or exactly 301 |
  | `!5xx` | Anything except 5xx |
  | `2xx\|!503` | Any 2xx, or anything that isn't 503 |

- **Body Matching** — optionally assert the response body `contains`, `equals`, or `not_equals` a value.

### Ping
Uses the system `ping` binary to check ICMP reachability. Works with hostnames and IP addresses.

### SSL Certificate
Performs a TLS handshake and reports certificate validity and days remaining until expiry. An alert is sent when expiry is within the configured threshold (default: 30 days).

### Heartbeat
A passive monitor — your service calls Observer Lite to signal it is alive. If no ping is received within `check_interval + grace_seconds`, the monitor goes down.

```
GET /api/heartbeat/{token}
POST /api/heartbeat/{token}
```

---

## Notifications

Notification channels are configured per-monitor. Each channel can independently toggle alerts for **down**, **recovery**, and **SSL expiry** events.

### Email (SMTP)

Configure via the **Notification Channels** page in the UI. Required fields: SMTP host, port, sender address, recipients list. TLS and STARTTLS are both supported. Passwords are stored with Fernet encryption (AES-128-CBC + HMAC-SHA256).

### Telegram

Create a bot via [@BotFather](https://t.me/botfather), then provide the `bot_token` and `chat_id` in the channel settings. A test notification can be sent from the UI before saving.

---

## Architecture

```
observer-lite/
├── crates/observer/  # Rust + Axum, one static binary
│   └── src/
│       ├── checkers/ # HTTP, Ping, SSL, Heartbeat probes
│       ├── routes/   # REST API
│       ├── scheduler.rs
│       └── services/ # Incidents, notifications, retention
└── frontend/         # Vue 3 + Vite + Tailwind, embedded in the binary
```

**Stack highlights:**
- **Backend:** Axum, Tokio, SQLx, SQLite WAL, rustls. One process owns the schedule.
- **Frontend:** Vue 3, Vite, Tailwind CSS, Pinia, uPlot. Icons and fonts are bundled; the UI makes no network calls of its own.
- **Transport:** Server-Sent Events for real-time updates.
- **Packaging:** Multi-stage Docker build. The runtime image is Alpine plus one static binary. Node and Python are not in the running container.

---

## API

Interactive API documentation is available at `/api/docs` (Swagger UI) and `/api/redoc` (ReDoc) when the server is running.

Key endpoint groups:

```
POST  /api/auth/login | refresh | logout | change-password
GET   /api/auth/me

GET|POST|PUT|DELETE  /api/monitors/{id}
POST  /api/monitors/{id}/pause | resume | check-now
GET   /api/monitors/{id}/stats | history | incidents

GET|POST|PUT|DELETE  /api/channels    + POST /{id}/test
GET|POST|PUT|DELETE  /api/incidents   + POST /{id}/acknowledge
GET|POST|PUT|DELETE  /api/maintenance
GET|POST|DELETE      /api/api-keys
GET|PUT              /api/settings

GET   /api/sse/dashboard    (SSE stream)
GET   /api/audit-log
GET   /api/export/monitors  |  POST /api/import/monitors
```

Authentication is via **JWT Bearer token** or an **`obs_`-prefixed API key**.

---

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full development guide including local setup, coding standards, and PR process.

```bash
# Frontend assets, then the binary (it embeds frontend/dist)
cd frontend && npm ci && npm run build && cd ..
cargo run --release

# Tests and the local quality gate (no push)
cargo test --workspace
./scripts/analyze.sh
```

---

## Docker Images

The GitHub Actions workflow publishes the single production image to GHCR. Successful CI runs on `main` update the moving `latest` and `main` tags. Versioned image tags are published only when pushing a matching `v*` Git tag, with the version read from the root [`VERSION`](VERSION) file.

Moving tags:

```text
ghcr.io/cagatayuresin/observer-lite:latest
ghcr.io/cagatayuresin/observer-lite:main
```

Release tags include:

```text
ghcr.io/cagatayuresin/observer-lite:0.3.0-alpha.1
ghcr.io/cagatayuresin/observer-lite:v0.3.0-alpha.1
```

This branch is the alpha. A successful CI run builds the image and checks `/api/health`. It does not push to GHCR. An alpha tag publishes only `0.3.0-alpha.1` and `v0.3.0-alpha.1`. `latest` and `0.3` stay on the stable release.

---

## Security

Observer Lite is designed with security in mind:

- Passwords hashed with **bcrypt** (cost 12)
- JWT access tokens expire in **15 minutes**; refresh tokens in **30 days**
- SMTP/Telegram credentials stored with **Fernet** symmetric encryption, key derived via PBKDF2
- API key raw values shown once; only **SHA-256 hash** stored
- Login endpoint rate-limited to **5 attempts / minute** per IP via slowapi

Please report vulnerabilities privately — see [SECURITY.md](SECURITY.md) for the responsible disclosure process.

---

## License

[MIT](LICENSE) © 2026 [Çağatay Üresin](https://github.com/cagatayuresin)
