"""APScheduler job functions that drive all periodic work.

Each function in this module is invoked directly by the scheduler; none of
them block the event loop — all blocking I/O is either awaited or dispatched
to a thread pool via :func:`asyncio.to_thread`.

Job registration happens in :mod:`app.scheduler.manager` (per-monitor) and
in :func:`app.main._register_background_jobs` (system-wide daily jobs).
"""

import logging
from datetime import datetime, timezone

from sqlalchemy import select

from app.checkers.base import CheckResult
from app.checkers.http_checker import http_check
from app.checkers.ping_checker import ping_check
from app.checkers.ssl_checker import ssl_check
from app.db.models import MaintenanceWindow, MaintenanceWindowMonitor, Monitor
from app.db.session import AsyncSessionLocal
from app.services.check_service import process_result
from app.services.notification_service import notify_ssl_warning
from app.services.retention_service import run_retention_cleanup

logger = logging.getLogger(__name__)


async def run_monitor_check(monitor_id: int) -> None:
    async with AsyncSessionLocal() as db:
        result = await db.execute(select(Monitor).where(Monitor.id == monitor_id))
        monitor = result.scalar_one_or_none()
        if monitor is None or not monitor.is_enabled:
            return

        # Check maintenance window
        if await _in_maintenance(db, monitor_id):
            return

        # Heartbeat monitor: check freshness, don't do outbound HTTP
        if monitor.monitor_type == "heartbeat":
            await _check_heartbeat(db, monitor)
            return

        # Perform the actual check
        check_result = await _perform_check(monitor)

        # SSL side-check
        if monitor.ssl_check_enabled and monitor.url.startswith("https://"):
            is_valid, days, err = await ssl_check(monitor.url)
            check_result.is_ssl_valid = is_valid
            check_result.ssl_expiry_days = days
            if days is not None and days <= monitor.ssl_expiry_warning_days:
                try:
                    await notify_ssl_warning(db, monitor, days)
                except Exception as e:
                    logger.error("SSL warning notification failed: %s", e)

        await process_result(db, monitor, check_result)


async def _perform_check(monitor: Monitor) -> CheckResult:
    if monitor.monitor_type == "ping":
        return await ping_check(monitor.url, monitor.timeout_seconds, monitor.response_time_warning_ms)

    method = {
        "http_get": "GET",
        "http_post": "POST",
        "http_head": "HEAD",
    }.get(monitor.monitor_type, "GET")

    return await http_check(
        url=monitor.url,
        method=method,
        headers_json=monitor.request_headers,
        body=monitor.request_body,
        expected_status_expr=monitor.expected_status_codes,
        expected_body_type=monitor.expected_body_type,
        expected_body_value=monitor.expected_body_value,
        timeout_seconds=monitor.timeout_seconds,
        response_time_warning_ms=monitor.response_time_warning_ms,
    )


async def _check_heartbeat(db, monitor: Monitor) -> None:
    now = datetime.now(timezone.utc)
    last_ping = monitor.heartbeat_last_ping
    grace = monitor.heartbeat_grace_seconds
    interval = monitor.check_interval_seconds

    if last_ping is None or (now - last_ping).total_seconds() > (interval + grace):
        result = CheckResult(status="down", error_message="Heartbeat not received in time")
    else:
        result = CheckResult(status="up")

    await process_result(db, monitor, result)


async def _in_maintenance(db, monitor_id: int) -> bool:
    now = datetime.now(timezone.utc)
    result = await db.execute(
        select(MaintenanceWindow)
        .join(MaintenanceWindowMonitor, MaintenanceWindowMonitor.window_id == MaintenanceWindow.id)
        .where(
            MaintenanceWindowMonitor.monitor_id == monitor_id,
            MaintenanceWindow.starts_at <= now,
            MaintenanceWindow.ends_at >= now,
        )
    )
    return result.scalar_one_or_none() is not None


async def run_daily_retention() -> None:
    try:
        await run_retention_cleanup()
    except Exception as e:
        logger.error("Retention cleanup failed: %s", e)


async def run_daily_ssl_scan() -> None:
    """Scan all HTTPS monitors with ssl_check_enabled for expiry warnings."""
    async with AsyncSessionLocal() as db:
        result = await db.execute(
            select(Monitor).where(
                Monitor.ssl_check_enabled.is_(True),
                Monitor.is_enabled.is_(True),
                Monitor.monitor_type.in_(["http_get", "http_post", "http_head"]),
            )
        )
        monitors = result.scalars().all()

        for monitor in monitors:
            if not monitor.url.startswith("https://"):
                continue
            try:
                is_valid, days, _ = await ssl_check(monitor.url)
                if days is not None and days <= monitor.ssl_expiry_warning_days:
                    await notify_ssl_warning(db, monitor, days)
            except Exception as e:
                logger.error("SSL scan failed for monitor %d: %s", monitor.id, e)
