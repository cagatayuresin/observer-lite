"""Notification dispatch: routes down/recovery/SSL alerts to the right channels.

Public entry points are :func:`notify_down`, :func:`notify_recovery`, and
:func:`notify_ssl_warning`.  Each builds channel-specific message payloads and
delegates to :func:`_dispatch`, which loads the monitor's assigned channels
from the database and fires all deliveries concurrently via
:func:`asyncio.gather`.
"""

import asyncio
import logging
from datetime import timedelta

from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db.models import Incident, Monitor, MonitorNotificationChannel, NotificationChannel
from app.services.email_service import send_email
from app.services.telegram_service import send_telegram

logger = logging.getLogger(__name__)


def _fmt_duration(seconds: int | None) -> str:
    if not seconds:
        return "unknown duration"
    td = timedelta(seconds=seconds)
    hours, remainder = divmod(td.seconds, 3600)
    minutes, secs = divmod(remainder, 60)
    if td.days:
        return f"{td.days}d {hours}h {minutes}m"
    if hours:
        return f"{hours}h {minutes}m"
    if minutes:
        return f"{minutes}m {secs}s"
    return f"{secs}s"


async def notify_down(db: AsyncSession, monitor: Monitor, incident: Incident) -> None:
    subject = f"[DOWN] {monitor.name} is DOWN"
    email_body = (
        f"<h2>Monitor Down: {monitor.name}</h2>"
        f"<p><b>URL:</b> {monitor.url}</p>"
        f"<p><b>Time:</b> {incident.started_at.strftime('%Y-%m-%d %H:%M:%S UTC')}</p>"
        f"<p><b>Reason:</b> {incident.root_cause or 'Unknown'}</p>"
    )
    telegram_msg = (
        f"🔴 <b>{monitor.name}</b> is DOWN\n"
        f"URL: {monitor.url}\n"
        f"Reason: {incident.root_cause or 'Unknown'}\n"
        f"Time: {incident.started_at.strftime('%H:%M:%S UTC')}"
    )
    await _dispatch(db, monitor.id, "down", subject, email_body, telegram_msg)


async def notify_recovery(db: AsyncSession, monitor: Monitor, incident: Incident) -> None:
    duration = _fmt_duration(incident.duration_seconds)
    subject = f"[UP] {monitor.name} recovered"
    email_body = (
        f"<h2>Monitor Recovered: {monitor.name}</h2>"
        f"<p><b>URL:</b> {monitor.url}</p>"
        f"<p><b>Recovered at:</b> {incident.resolved_at.strftime('%Y-%m-%d %H:%M:%S UTC')}</p>"
        f"<p><b>Downtime:</b> {duration}</p>"
    )
    telegram_msg = (
        f"🟢 <b>{monitor.name}</b> is UP again\n"
        f"URL: {monitor.url}\n"
        f"Downtime: {duration}\n"
        f"Recovered: {incident.resolved_at.strftime('%H:%M:%S UTC')}"
    )
    await _dispatch(db, monitor.id, "recovery", subject, email_body, telegram_msg)


async def notify_ssl_warning(db: AsyncSession, monitor: Monitor, days_left: int) -> None:
    subject = f"[SSL] {monitor.name} certificate expires in {days_left} days"
    email_body = (
        f"<h2>SSL Warning: {monitor.name}</h2>"
        f"<p>Certificate expires in <b>{days_left} days</b>.</p>"
        f"<p><b>URL:</b> {monitor.url}</p>"
    )
    telegram_msg = (
        f"⚠️ <b>SSL Warning</b>: {monitor.name}\n"
        f"Certificate expires in {days_left} days\n"
        f"URL: {monitor.url}"
    )
    await _dispatch(db, monitor.id, "ssl_warn", subject, email_body, telegram_msg)


async def _dispatch(
    db: AsyncSession,
    monitor_id: int,
    event: str,  # down|recovery|ssl_warn
    subject: str,
    email_body: str,
    telegram_msg: str,
) -> None:
    result = await db.execute(
        select(MonitorNotificationChannel, NotificationChannel)
        .join(NotificationChannel, MonitorNotificationChannel.channel_id == NotificationChannel.id)
        .where(
            MonitorNotificationChannel.monitor_id == monitor_id,
            NotificationChannel.is_enabled.is_(True),
        )
    )
    rows = result.all()

    tasks = []
    for mnc, channel in rows:
        if event == "down" and not mnc.on_down:
            continue
        if event == "recovery" and not mnc.on_recovery:
            continue
        if event == "ssl_warn" and not mnc.on_ssl_warn:
            continue

        if channel.channel_type == "email":
            tasks.append(send_email(channel.config, subject, email_body))
        elif channel.channel_type == "telegram":
            tasks.append(send_telegram(channel.config, telegram_msg))

    if tasks:
        results = await asyncio.gather(*tasks, return_exceptions=True)
        for r in results:
            if isinstance(r, Exception):
                logger.error("Notification delivery failed: %s", r)
