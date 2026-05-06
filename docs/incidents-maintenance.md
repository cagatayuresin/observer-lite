---
title: Incidents and Maintenance
permalink: /incidents-maintenance/
---

# Incidents and Maintenance

Observer Lite tracks outages as incidents and supports maintenance windows to suppress expected alert noise.

## Incident Lifecycle

An incident starts when a monitor has enough consecutive failures to reach its retry threshold.

Example:

| Setting | Value |
| --- | --- |
| Check interval | `60` seconds |
| Retry count | `3` |

If the monitor fails three checks in a row, Observer Lite opens an incident and sends a down notification.

The incident closes when a later check succeeds.

## Incident Fields

| Field | Meaning |
| --- | --- |
| Monitor | Monitor that failed. |
| Started at | Time the incident opened. |
| Resolved at | Time the monitor recovered. |
| Duration | Downtime length in seconds. |
| Root cause | Error message captured from the failed check. |
| Acknowledged by | User who acknowledged the incident, when applicable. |

## Acknowledgement

Acknowledgement records that a human has seen the incident. It does not close the incident.

Use acknowledgement for:

- Handoffs between responders
- Reducing duplicate investigation
- Recording that the outage is known

## Maintenance Windows

Maintenance windows describe planned downtime.

Fields:

| Field | Description |
| --- | --- |
| Name | Human-readable reason, such as `Database upgrade`. |
| Starts at | Beginning of the maintenance period. |
| Ends at | End of the maintenance period. |
| Repeat cron | Optional cron expression for repeating windows. |
| Suppress alerts | Whether matching checks should suppress notifications. |
| Monitor IDs | Monitors affected by the window. |

## Creating a Window

1. Open **Maintenance**.
2. Create a new window.
3. Choose start and end times.
4. Select affected monitors.
5. Enable alert suppression if the downtime is expected.

## How Checks Behave During Maintenance

When a monitor is inside an active maintenance window with suppression enabled, scheduled checks are skipped for alerting purposes. This prevents planned downtime from opening noisy incidents.

## Suggested Practices

- Give maintenance windows descriptive names.
- Keep windows as short as practical.
- Use monitor groups to make selection easier.
- Create windows before deploying risky infrastructure changes.
- Review old windows periodically.

## What Maintenance Does Not Do

Maintenance windows do not pause the whole application.

They do not:

- Stop the web UI
- Disable user login
- Delete check history
- Change monitor configuration
- Silence unrelated monitors
