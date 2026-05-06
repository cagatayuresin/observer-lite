---
title: Troubleshooting
permalink: /troubleshooting/
---

# Troubleshooting

Use this page when Observer Lite does not start, checks do not run, or alerts do not arrive.

## Container Exits Immediately

Check logs:

```bash
docker logs observer-lite
```

Common causes:

| Symptom | Fix |
| --- | --- |
| `SECRET_KEY is required` | Set `SECRET_KEY` to a long random value. |
| SQLite permission error | Verify the `/data` mount is writable by the container. |
| Migration failure | Check whether the database file is corrupt or from an unsupported future version. |

## Cannot Log In

Default first login:

| Field | Value |
| --- | --- |
| Username | `admin` |
| Password | `admin` |

If this is not a fresh database, the password may already have been changed.

If the account exists but cannot log in:

1. Check that the container is using the expected `/data` volume.
2. Confirm the browser is reaching the correct host.
3. Review container logs for authentication errors.

## Checks Are Not Running

Check logs for scheduler startup:

```text
Scheduler started
Scheduled N monitor jobs
```

If monitors are not scheduled:

- Confirm each monitor is enabled.
- Confirm the app is running with one worker.
- Check whether the monitor is inside a maintenance window.
- Use **Check now** from the monitor page.

## HTTP Monitor Is Down Unexpectedly

Review:

- URL is reachable from inside the container.
- Expected status expression matches the real response.
- Custom headers are valid JSON.
- Body match mode and value match the response.
- Timeout is long enough for the target.

Test from inside a container network when possible:

```bash
curl -i https://example.com/health
```

## Ping Monitor Fails

Ping depends on ICMP reachability.

Some hosts and networks block ICMP even when HTTP is healthy. If that is expected, use an HTTP monitor instead.

Also verify that the Docker image includes `iputils-ping`; the official image does.

## SSL Warning Does Not Send

Check:

- URL starts with `https://`.
- SSL checking is enabled on the monitor.
- Expiry days are below the configured warning threshold.
- The assigned notification channel has SSL warning enabled.
- The channel itself is enabled.

## Email Test Fails

Common causes:

| Cause | Fix |
| --- | --- |
| Wrong SMTP port | Try `587` for STARTTLS or `465` for TLS. |
| Provider blocks password auth | Create an app password or SMTP token. |
| From address rejected | Use a sender address allowed by the provider. |
| Network blocked | Verify the host can reach the SMTP server. |

Check logs:

```bash
docker logs observer-lite
```

## Telegram Test Fails

Check:

- Bot token is correct.
- Bot was added to the chat or group.
- The chat ID is correct.
- The chat has received at least one message after the bot was added.

## Dashboard Does Not Update Live

The dashboard uses Server-Sent Events.

If live updates do not work:

- Confirm the browser can reach `/api/sse/dashboard`.
- Check reverse proxy buffering settings.
- Keep the app at one Uvicorn worker.

Nginx may need buffering disabled for SSE:

```nginx
proxy_buffering off;
proxy_cache off;
```

## GitHub Pages Site Does Not Publish

Check:

1. Repository **Settings** -> **Pages** uses **GitHub Actions**.
2. The **Pages** workflow has permission to deploy.
3. The workflow completed successfully.
4. The public URL uses the project path:

```text
https://cagatayuresin.github.io/observer-lite/
```

## GHCR Image Is Not Public

After the first package is created:

1. Open the repository on GitHub.
2. Open **Packages**.
3. Choose `observer-lite`.
4. Update package visibility to public if needed.

The workflow can push the image, but package visibility is controlled in GitHub package settings.
