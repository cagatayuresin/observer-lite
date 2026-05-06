---
title: Operations
permalink: /operations/
---

# Operations

This page covers day-two operations: releases, backups, logs, upgrades, and health checks.

## GitHub Pages Documentation

The documentation site is built from the `docs/` directory and published by the Pages workflow.

After pushing to `main`, open the repository settings and confirm:

1. **Settings** -> **Pages**
2. Source is **GitHub Actions**
3. The latest **Pages** workflow completed successfully

The public URL is expected to be:

```text
https://cagatayuresin.github.io/observer-lite/
```

## GHCR Image Publishing

Docker images publish to GitHub Container Registry from the Docker workflow.

Triggers:

- Push to `main`
- Push a tag matching `v*`
- Manual workflow dispatch

Version tags come from the root `VERSION` file.

Release flow:

```bash
printf "0.1.1\n" > VERSION
git add VERSION
git commit -m "Bump version to 0.1.1"
git tag v0.1.1
git push origin main v0.1.1
```

The workflow rejects a tag if it does not match `VERSION`.

## Logs

Docker:

```bash
docker logs -f observer-lite
```

Docker Compose:

```bash
docker compose logs -f observer-lite
```

Look for:

- Migration errors
- Scheduler startup lines
- Notification delivery failures
- SQLite permission errors

## Health Checks

The image healthcheck calls:

```text
/api/docs
```

Manual check:

```bash
curl -fsS http://localhost:3000/api/docs >/dev/null
```

## Backups

The important runtime file is:

```text
/data/observer.db
```

Recommended backup process:

1. Stop the container.
2. Copy the database file from the mounted volume.
3. Start the container again.
4. Periodically test restore in a separate environment.

For higher availability, use a SQLite-aware backup mechanism instead of copying a live file.

## Restore

1. Stop Observer Lite.
2. Replace `/data/observer.db` with the backup.
3. Start Observer Lite.
4. Check logs for migration output.
5. Log in and verify monitors, users, and notification channels.

## Upgrades

Pull the new image:

```bash
docker pull ghcr.io/cagatayuresin/observer-lite:latest
```

Docker Compose:

```bash
docker compose pull
docker compose up -d
```

Migrations run automatically during container startup.

## Data Retention

Observer Lite runs a daily retention job. The default retention period is 90 days.

Change it from the settings UI or API by updating:

```text
data_retention_days
```

## Security Operations

- Use HTTPS in front of the app.
- Store `SECRET_KEY` in a secret manager or protected environment.
- Keep the default admin account password changed.
- Review API keys periodically.
- Remove unused notification channels.
- Keep the container image updated.

## Scaling Notes

Observer Lite is designed for a single process.

Do not scale the same database behind multiple app containers unless scheduler ownership and event fan-out are redesigned. Multiple active schedulers can duplicate checks and notifications.
