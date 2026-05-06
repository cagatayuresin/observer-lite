---
title: Notifications
permalink: /notifications/
---

# Notifications

Notification channels deliver down, recovery, and SSL expiry alerts. Channels are created once and assigned to monitors as needed.

## Event Types

| Event | When it is sent |
| --- | --- |
| Down | A monitor reaches its retry threshold and an incident opens. |
| Recovery | A monitor returns to a healthy state after an incident. |
| SSL warning | A certificate is valid but near its configured expiry threshold. |

Each monitor-channel assignment can choose which event types are enabled.

## Email Channels

Email channels use SMTP.

Required values:

| Field | Description |
| --- | --- |
| SMTP host | Mail server hostname. |
| SMTP port | Usually `587`, `465`, or `25`. |
| SMTP user | Username for authentication, if required. |
| SMTP password | Stored encrypted by Observer Lite. |
| From address | Sender address shown in alerts. |
| Recipients | One or more destination email addresses. |
| TLS settings | Choose the mode required by your provider. |

Common ports:

| Port | Typical use |
| --- | --- |
| `587` | STARTTLS submission. |
| `465` | TLS from connection start. |
| `25` | Plain SMTP or relay inside a private network. |

After saving the channel, use the **Test** action before assigning it to production monitors.

## Telegram Channels

Telegram channels require a bot token and chat ID.

Setup flow:

1. Open Telegram and talk to `@BotFather`.
2. Create a bot and copy the bot token.
3. Add the bot to the destination chat or group.
4. Send at least one message in the chat.
5. Get the chat ID using the Telegram Bot API or your preferred helper tool.
6. Save the channel in Observer Lite.
7. Use the **Test** action.

Required values:

| Field | Description |
| --- | --- |
| Bot token | Token from `@BotFather`. |
| Chat ID | Numeric or string chat identifier. |

## Assign Channels to Monitors

Create the channel first, then assign it from the monitor UI.

For each assignment, choose:

- Notify on down
- Notify on recovery
- Notify on SSL warning

This lets you route noisy events differently. For example, a team channel can receive down and recovery alerts while a security mailbox receives only SSL warnings.

## Alert Lifecycle

Observer Lite avoids sending a down notification for every failed check.

The simplified flow is:

1. A check fails.
2. `consecutive_failures` increases.
3. Once failures reach `retry_count`, an incident opens.
4. A down notification is sent once for that incident.
5. When the monitor recovers, the incident closes.
6. A recovery notification is sent once.

## Message Content

Down alerts include:

- Monitor name
- URL
- Failure time
- Root cause or error message

Recovery alerts include:

- Monitor name
- URL
- Recovery time
- Downtime duration

SSL warning alerts include:

- Monitor name
- URL
- Days remaining before certificate expiry

## Delivery Failures

If one channel fails, Observer Lite logs the error and continues processing the check result. A failed notification does not roll back incident state.

Check container logs when a test notification fails:

```bash
docker logs observer-lite
```
