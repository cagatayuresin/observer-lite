<p align="center">
  <img src="frontend/public/logo.svg" width="96" height="96" alt="Observer Lite" />
</p>

<h1 align="center">Observer Lite</h1>

<p align="center">
  Lightweight, self-hosted uptime monitoring with email &amp; Telegram alerts,<br/>
  SSL certificate tracking, real-time dashboard, and a single Docker image.
</p>

<p align="center">
  <a href="https://github.com/cagatayuresin/observer-lite/releases/tag/v0.1.0">
    <img src="https://img.shields.io/badge/version-0.1.0-blue?style=flat-square" alt="Version 0.1.0" />
  </a>
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/license-MIT-green?style=flat-square" alt="MIT License" />
  </a>
  <img src="https://img.shields.io/badge/coverage-80%25-brightgreen?style=flat-square" alt="Test Coverage 80%" />
  <img src="https://img.shields.io/badge/python-3.12-3776ab?style=flat-square&logo=python&logoColor=white" alt="Python 3.12" />
  <img src="https://img.shields.io/badge/vue-3-42b883?style=flat-square&logo=vue.js&logoColor=white" alt="Vue 3" />
  <a href="https://github.com/cagatayuresin/observer-lite/pkgs/container/observer-lite">
    <img src="https://img.shields.io/badge/docker-ghcr.io-2496ed?style=flat-square&logo=docker&logoColor=white" alt="Docker Image on GHCR" />
  </a>
</p>

---

## Overview

**Observer Lite** is a self-hosted uptime monitoring tool built for simplicity and efficiency. It monitors up to 500+ endpoints simultaneously, tracks SSL certificate expiry, sends instant alerts via email and Telegram, and delivers a real-time dark-mode dashboard — all from a single Docker image with no external database required.

Think of it as a lighter, more flexible alternative to Uptime Kuma — with a built-in status code DSL, response body matching, API keys, audit logging, and configurable data retention.

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
| **User Management** | Roles: `superadmin`, `admin`, `viewer`; forced password change on first login |
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
  -e DATABASE_URL="sqlite+aiosqlite:////data/observer.db" \
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
      DATABASE_URL: "sqlite+aiosqlite:////data/observer.db"
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
| `DATABASE_URL` | No | `sqlite+aiosqlite:////data/observer.db` | Full SQLAlchemy async connection URL. |
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
├── backend/          # Python 3.12 + FastAPI (async)
│   └── app/
│       ├── checkers/ # HTTP, Ping, SSL, Heartbeat probes
│       ├── routers/  # REST API endpoints
│       ├── scheduler/# APScheduler engine + per-monitor jobs
│       ├── services/ # Business logic (incidents, notifications, retention)
│       ├── sse/      # Server-Sent Events broadcaster
│       └── utils/    # Crypto, pagination
└── frontend/         # Vue 3 + Vite + Tailwind CSS + Pinia
```

**Stack highlights:**
- **Backend:** FastAPI, SQLAlchemy 2.0 async, aiosqlite, APScheduler 3.x (SQLAlchemyJobStore), aiosmtplib, python-jose, bcrypt, slowapi
- **Frontend:** Vue 3, Vite, Tailwind CSS, Pinia, uPlot (lightweight charts, ~40 KB)
- **Transport:** Server-Sent Events for real-time updates (no WebSocket needed)
- **Packaging:** Multi-stage Docker build — Node.js build + Python 3.12-slim runtime, single image

> **Important:** The application must run with `--workers 1`. The APScheduler and SSE broadcaster are in-memory singletons; multiple workers would duplicate checks and miss events.

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
# Backend
cd backend
uv venv && uv pip install -e ".[dev]"
alembic upgrade head
uvicorn app.main:app --reload --workers 1

# Frontend (separate terminal)
cd frontend
npm install
npm run dev

# Tests
cd backend
pytest tests/ --cov=app -q
```

---

## Docker Images

The GitHub Actions workflow publishes the single production image to GHCR on pushes to `main`, `v*` tags, or manual workflow runs. Image version tags are read from the root [`VERSION`](VERSION) file, so bump that file before cutting a release.

Published tags include:

```text
ghcr.io/cagatayuresin/observer-lite:latest
ghcr.io/cagatayuresin/observer-lite:0.1.0
ghcr.io/cagatayuresin/observer-lite:v0.1.0
ghcr.io/cagatayuresin/observer-lite:0.1
```

When pushing a Git tag, use the `v`-prefixed form that matches `VERSION`, for example `v0.1.0`.

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
