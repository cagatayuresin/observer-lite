---
title: Overview
permalink: /
---

<section class="hero">
  <h1>Observer Lite Documentation</h1>
  <p>Observer Lite is a lightweight, self-hosted uptime monitor with HTTP, ping, SSL, and heartbeat checks, alert routing, incident tracking, maintenance windows, API keys, and a bundled Vue dashboard.</p>
</section>

<div class="callout-grid">
  <div class="callout">
    <strong>Single image</strong>
    Run the backend, dashboard, scheduler, migrations, and static assets from one Docker image.
  </div>
  <div class="callout">
    <strong>SQLite first</strong>
    Store runtime data in a mounted `/data` volume without operating an external database.
  </div>
  <div class="callout">
    <strong>Alert aware</strong>
    Send down, recovery, and SSL expiry notifications by email or Telegram.
  </div>
</div>

## What You Can Monitor

Observer Lite supports active and passive checks:

| Monitor type | Use it for |
| --- | --- |
| HTTP GET / POST / HEAD | Websites, APIs, webhooks, and status endpoints |
| Ping | Hosts where ICMP reachability is a useful signal |
| SSL certificate | Certificate validity and expiry warnings for HTTPS targets |
| Heartbeat | Jobs and services that should call Observer Lite on a schedule |

## Main Concepts

**Monitor**  
A target to check. Each monitor stores its URL, type, interval, timeout, retry settings, status matching rules, alert behavior, and current live state.

**Check result**  
A single probe result. It records status, response time, HTTP status code, SSL state, and error details.

**Incident**  
An outage lifecycle created after a monitor exceeds its retry threshold. Incidents can be acknowledged and are closed automatically when the monitor recovers.

**Notification channel**  
An email or Telegram destination that can be assigned to one or more monitors.

**Maintenance window**  
A planned downtime period that suppresses alert noise for selected monitors.

## Recommended Reading Path

1. Start with [Quick Start](quick-start/) to run the app locally.
2. Review [Configuration](configuration/) before deploying a production instance.
3. Use [Monitors](monitors/) to model HTTP, ping, SSL, and heartbeat checks.
4. Configure [Notifications](notifications/) so outages reach the right people.
5. Keep [Operations](operations/) and [Troubleshooting](troubleshooting/) handy after deployment.

## Production Checklist

- Set a long random `SECRET_KEY`.
- Mount `/data` to persistent storage.
- Keep `--workers 1`; the scheduler and SSE broadcaster are process-local.
- Put the app behind HTTPS when exposed to the internet.
- Change the default `admin` password on first login.
- Back up the SQLite database file regularly.
- Configure at least one notification channel and test it from the UI.
