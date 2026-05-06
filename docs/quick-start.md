---
title: Quick Start
permalink: /quick-start/
---

# Quick Start

This page gets Observer Lite running with Docker, then walks through the first login and first monitor.

## Requirements

- Docker Engine 24+ or Docker Desktop
- A host port for the web UI, usually `3000`
- A persistent volume for `/data`
- A long random `SECRET_KEY`

Generate a secret:

```bash
openssl rand -hex 32
```

## Run With Docker

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

Open `http://localhost:3000`.

Default credentials:

| Field | Value |
| --- | --- |
| Username | `admin` |
| Password | `admin` |

You will be asked to set a new password immediately.

## Run With Docker Compose

Create `docker-compose.yml`:

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
      PORT: "3000"
      SECRET_KEY: "replace-with-a-long-random-secret"
      DATABASE_PATH: "/data/observer.db"
      DATABASE_URL: "sqlite+aiosqlite:////data/observer.db"

volumes:
  observer_data:
```

Start it:

```bash
docker compose up -d
```

View logs:

```bash
docker compose logs -f observer-lite
```

## Create Your First HTTP Monitor

1. Log in to the dashboard.
2. Open **Monitors**.
3. Choose **Create Monitor**.
4. Use these values:

| Field | Example |
| --- | --- |
| Name | `Website homepage` |
| Type | `HTTP GET` |
| URL | `https://example.com` |
| Interval | `60` seconds |
| Timeout | `10` seconds |
| Expected status | `2xx` |

Save the monitor. The scheduler will start checking it automatically.

## Create a Notification Channel

1. Open **Notifications**.
2. Add an email or Telegram channel.
3. Use the **Test** action before assigning it to production monitors.
4. Assign the channel to a monitor and choose which events should notify:
   - Down
   - Recovery
   - SSL warning

## Verify the Container

Check that the app responds:

```bash
curl -fsS http://localhost:3000/api/docs >/dev/null
```

Check the image version label:

```bash
docker image inspect ghcr.io/cagatayuresin/observer-lite:latest \
{% raw %}
  --format '{{ index .Config.Labels "org.opencontainers.image.version" }}'
{% endraw %}
```

## Stop and Remove

```bash
docker rm -f observer-lite
```

Remove the data volume only when you intentionally want to delete all persisted data:

```bash
docker volume rm observer_data
```
