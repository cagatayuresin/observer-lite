---
title: API
permalink: /api/
---

# API

Observer Lite exposes a REST API under `/api`. The running application also provides interactive Swagger documentation at `/api/docs`.

## Authentication

The API accepts:

- JWT access tokens returned by `/api/auth/login`
- `obs_` prefixed API keys created in the UI or API

Use credentials as a Bearer token:

```bash
curl -H "Authorization: Bearer YOUR_TOKEN" \
  https://observer.example.com/api/auth/me
```

## Login

```bash
curl -sS -X POST https://observer.example.com/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin"}'
```

The response includes an access token and refresh token.

## API Keys

Create a key:

```bash
curl -sS -X POST https://observer.example.com/api/api-keys \
  -H "Authorization: Bearer ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"automation"}'
```

The raw key is returned once. Store it immediately.

Use it:

```bash
curl -H "Authorization: Bearer obs_YOUR_KEY" \
  https://observer.example.com/api/monitors
```

## Endpoint Groups

| Area | Endpoints |
| --- | --- |
| Auth | `/api/auth/login`, `/api/auth/refresh`, `/api/auth/logout`, `/api/auth/me`, `/api/auth/change-password` |
| Users | `/api/users` |
| Monitors | `/api/monitors`, `/api/monitors/{id}` |
| Monitor actions | `/api/monitors/{id}/pause`, `/api/monitors/{id}/resume`, `/api/monitors/{id}/check-now` |
| Results | `/api/check-results` |
| Incidents | `/api/incidents`, `/api/incidents/{id}/acknowledge` |
| Notifications | `/api/channels`, `/api/channels/{id}/test` |
| Maintenance | `/api/maintenance` |
| Settings | `/api/settings` |
| API keys | `/api/api-keys` |
| Audit log | `/api/audit-log` |
| Import/export | `/api/export/monitors`, `/api/import/monitors` |
| Realtime | `/api/sse/dashboard` |
| Heartbeat | `/api/heartbeat/{token}` |

## Create a Monitor

```bash
curl -sS -X POST https://observer.example.com/api/monitors \
  -H "Authorization: Bearer ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Homepage",
    "url": "https://example.com",
    "monitor_type": "http_get",
    "check_interval_seconds": 60,
    "timeout_seconds": 10,
    "retry_count": 3,
    "expected_status_codes": "2xx"
  }'
```

## List Check Results

```bash
curl -H "Authorization: Bearer ACCESS_TOKEN" \
  "https://observer.example.com/api/check-results?monitor_id=1&limit=50"
```

Optional query parameters:

| Parameter | Description |
| --- | --- |
| `monitor_id` | Limit results to a monitor. |
| `from` | ISO datetime lower bound. |
| `to` | ISO datetime upper bound. |
| `limit` | Maximum results, up to `1000`. |

## Export Monitors

```bash
curl -H "Authorization: Bearer ACCESS_TOKEN" \
  https://observer.example.com/api/export/monitors
```

The export includes portable monitor configuration, not runtime state.

## Import Monitors

```bash
curl -sS -X POST https://observer.example.com/api/import/monitors \
  -H "Authorization: Bearer ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  --data @monitors.json
```

Unknown keys are ignored during import to avoid writing unsafe runtime fields such as IDs or current status.

## Server-Sent Events

The dashboard uses SSE:

```text
GET /api/sse/dashboard
```

Events are emitted when check results are processed. Use the dashboard as the primary SSE client unless you are building an integration.
