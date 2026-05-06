---
title: Configuration
permalink: /configuration/
---

# Configuration

Observer Lite is configured through environment variables. Most application settings can also be managed in the dashboard after the first login.

## Environment Variables

| Variable | Required | Default | Description |
| --- | --- | --- | --- |
| `PORT` | No | `3000` | HTTP port used by Uvicorn inside the container. |
| `SECRET_KEY` | Yes | none in Docker | Signing and encryption root secret. Use a long random value. |
| `DATABASE_PATH` | No | `/data/observer.db` | SQLite database path. |
| `DATABASE_URL` | No | `sqlite+aiosqlite:////data/observer.db` | Async SQLAlchemy database URL. |
| `DEBUG` | No | `false` | Enables extra debug behavior when set to a boolean true value. |

## Required Secret

`SECRET_KEY` is required in the Docker image. It protects JWT signing and credential encryption.

Use this pattern:

```bash
SECRET_KEY="$(openssl rand -hex 32)"
```

Do not rotate `SECRET_KEY` casually on an existing installation. Changing it can invalidate tokens and make previously encrypted notification credentials unreadable.

## Data Storage

The production image uses SQLite by default:

```text
/data/observer.db
```

Mount `/data` to persistent storage:

```bash
docker run -v observer_data:/data ...
```

Back up the database file while the app is stopped, or use a SQLite-safe backup process.

## Application Settings

The dashboard stores application settings in the `app_settings` table.

Common settings:

| Setting | Purpose |
| --- | --- |
| `data_retention_days` | Number of days to keep check result history. |
| `app_base_url` | Public URL used in generated links and notifications. |
| SMTP settings | Saved by the notification/settings UI for email delivery. |

Sensitive settings are masked when fetched by the API.

## Networking

Expose the container port you want users to reach:

```yaml
ports:
  - "3000:3000"
```

If Observer Lite runs behind a reverse proxy, terminate HTTPS at the proxy and forward traffic to the container.

Example Nginx location:

```nginx
location / {
  proxy_pass http://127.0.0.1:3000;
  proxy_http_version 1.1;
  proxy_set_header Host $host;
  proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
  proxy_set_header X-Forwarded-Proto $scheme;
}
```

## Worker Count

Run Observer Lite with one worker only.

The scheduler and SSE broadcaster are process-local. Multiple workers can duplicate checks, split dashboard events, and create confusing incident behavior.

The bundled entrypoint already starts Uvicorn with:

```bash
--workers 1
```

## GitHub Container Registry Tags

Published images use tags derived from the root `VERSION` file:

| Tag | Meaning |
| --- | --- |
| `latest` | Latest build from the default branch. |
| `0.1.0` | Exact version from `VERSION`. |
| `v0.1.0` | Git-style version tag. |
| `0.1` | Major/minor convenience tag. |

When creating a release tag, make it match `VERSION`:

```bash
git tag v0.1.0
git push origin v0.1.0
```
