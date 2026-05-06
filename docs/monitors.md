---
title: Monitors
permalink: /monitors/
---

# Monitors

A monitor defines what Observer Lite checks, how often it checks, what counts as healthy, and when alerts should be sent.

## Shared Fields

| Field | Description |
| --- | --- |
| Name | Human-readable label shown in the dashboard and notifications. |
| URL | Target URL, hostname, or heartbeat identifier. |
| Type | HTTP GET, HTTP POST, HTTP HEAD, ping, or heartbeat. |
| Interval | Seconds between scheduled checks. |
| Timeout | Maximum time to wait before marking a check down. |
| Retry count | Number of consecutive failures required before an incident opens. |
| Retry interval | Delay between retries where applicable. |
| Alert cooldown | Minimum time between repeated alerts for the same outage. |
| Alerts enabled | Master switch for notifications on that monitor. |

## HTTP Monitors

HTTP monitors send a request and evaluate both status code and optional response body rules.

Supported methods:

- `HTTP GET`
- `HTTP POST`
- `HTTP HEAD`

Example:

| Field | Value |
| --- | --- |
| URL | `https://api.example.com/health` |
| Expected status codes | `2xx` |
| Timeout | `10` |
| Response time warning | `2000` ms |

## Status Code DSL

The expected status field accepts a small expression language.

| Expression | Meaning |
| --- | --- |
| `200` | Exactly HTTP 200. |
| `2xx` | Any 200-299 response. |
| `200|301` | HTTP 200 or 301. |
| `2xx|301` | Any 2xx response or 301. |
| `!5xx` | Anything except 500-599. |
| `2xx|!503` | Any 2xx response, or anything that is not 503. |

Keep expressions simple. For most public websites and APIs, `2xx` is the clearest rule.

## Body Matching

Body matching is optional.

| Mode | Behavior |
| --- | --- |
| `contains` | Response body must contain the expected text. |
| `equals` | Response body must exactly equal the expected text. |
| `not_equals` | Response body must not exactly equal the expected text. |

Useful examples:

| Scenario | Mode | Value |
| --- | --- | --- |
| Health endpoint returns JSON | `contains` | `"status":"ok"` |
| Static status page | `contains` | `All systems operational` |
| Maintenance placeholder detection | `not_equals` | `maintenance` |

## Request Headers and Body

HTTP monitors can store custom headers as a JSON string.

```json
{
  "Authorization": "Bearer example-token",
  "Accept": "application/json"
}
```

For `HTTP POST`, set the request body to the payload expected by the target.

```json
{"probe":"observer-lite"}
```

## Ping Monitors

Ping monitors call the system `ping` binary inside the container.

Use ping when ICMP reachability is the signal you care about:

```text
example.com
```

or:

```text
https://example.com
```

Observer Lite extracts the hostname and runs one ICMP request.

## SSL Certificate Checks

SSL checks are available for HTTPS URLs. They report:

- Whether the certificate is valid
- Days remaining before expiry
- Warning state when expiry is within the configured threshold

Default warning threshold:

```text
30 days
```

SSL warning notifications are controlled per notification channel assignment.

## Heartbeat Monitors

Heartbeat monitors are passive. Your service calls Observer Lite to prove that it is still alive.

When you create a heartbeat monitor, Observer Lite generates a token. Use either endpoint:

```text
GET /api/heartbeat/{token}
POST /api/heartbeat/{token}
```

Example cron job:

```bash
curl -fsS https://observer.example.com/api/heartbeat/YOUR_TOKEN >/dev/null
```

Observer Lite marks the monitor down if no heartbeat arrives within:

```text
check_interval_seconds + heartbeat_grace_seconds
```

Heartbeat monitors are useful for:

- Cron jobs
- Backup scripts
- Queue workers
- Batch imports
- Small internal services without HTTP health endpoints

## Groups

Groups organize monitor lists and dashboard filters. Deleting a group does not delete its monitors.

Suggested groups:

- Production
- Internal
- Customer-facing
- Jobs
- Infrastructure

## Manual Checks

Use **Check now** from the monitor detail page to run a check immediately without waiting for the next scheduler interval.
