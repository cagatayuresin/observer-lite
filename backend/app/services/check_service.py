"""Core check result processing and incident state machine.

This is the heart of Observer Lite's alerting logic.  Every probe result
passes through :func:`process_result`, which:

1. Persists the :class:`~app.db.models.CheckResult` row.
2. Updates the denormalised status fields on the
   :class:`~app.db.models.Monitor`.
3. Manages the incident lifecycle:
   - Opens an :class:`~app.db.models.Incident` when
     ``consecutive_failures >= retry_count``.
   - Sends a *down* notification once per incident (``notification_sent``
     flag prevents duplicates).
   - Closes the incident and sends a *recovery* notification on the first
     successful check after an outage.
4. Broadcasts an SSE event so connected dashboards update in real-time.
"""

import logging
from datetime import UTC, datetime

from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.checkers.base import CheckResult
from app.db.models import CheckResult as DBCheckResult
from app.db.models import Incident, Monitor
from app.services.notification_service import notify_down, notify_recovery
from app.sse.broadcaster import broadcaster
from app.utils.time import ensure_utc

logger = logging.getLogger(__name__)


async def process_result(db: AsyncSession, monitor: Monitor, result: CheckResult) -> None:
    """Persist *result* and advance the incident state machine for *monitor*.

    This function is the single entry point for all checker output.  It
    commits the database changes before broadcasting the SSE event, so that
    any client that immediately fetches the monitor state after the event
    will see the updated values.

    Args:
        db: An open async database session.
        monitor: The :class:`~app.db.models.Monitor` that was just checked
            (loaded in the same session so that mutations are tracked).
        result: The :class:`~app.checkers.base.CheckResult` produced by the
            appropriate checker.
    """
    # Persist check result
    db_result = DBCheckResult(
        monitor_id=monitor.id,
        checked_at=result.checked_at or datetime.now(UTC),
        status=result.status,
        response_time_ms=result.response_time_ms,
        status_code=result.status_code,
        is_ssl_valid=result.is_ssl_valid,
        ssl_expiry_days=result.ssl_expiry_days,
        error_message=result.error_message,
    )
    db.add(db_result)

    is_failure = result.status == "down"
    if is_failure:
        monitor.consecutive_failures += 1
    else:
        monitor.consecutive_failures = 0

    prev_status = monitor.current_status
    _apply_live_state(monitor, result)

    open_incident = await _get_open_incident(db, monitor.id)
    if is_failure:
        await _record_failure(db, monitor, result, open_incident)
    else:
        await _record_recovery(db, monitor, open_incident)

    await db.commit()

    broadcaster.publish("monitor.check_result", {
        "monitor_id": monitor.id,
        "status": result.status,
        "prev_status": prev_status,
        "response_time_ms": result.response_time_ms,
        "checked_at": (result.checked_at or datetime.now(UTC)).isoformat(),
    })


def _apply_live_state(monitor: Monitor, result: CheckResult) -> None:
    monitor.current_status = result.status
    monitor.last_checked_at = result.checked_at or datetime.now(UTC)
    monitor.last_response_time_ms = result.response_time_ms
    monitor.updated_at = datetime.now(UTC)


async def _record_failure(
    db: AsyncSession,
    monitor: Monitor,
    result: CheckResult,
    open_incident: Incident | None,
) -> None:
    if monitor.consecutive_failures < monitor.retry_count:
        return
    incident = open_incident
    if incident is None:
        incident = Incident(
            monitor_id=monitor.id,
            started_at=datetime.now(UTC),
            root_cause=result.error_message,
        )
        db.add(incident)
        await db.flush()
    await _notify_down_once(db, monitor, incident)


async def _notify_down_once(db: AsyncSession, monitor: Monitor, incident: Incident) -> None:
    if not monitor.alerts_enabled or incident.notification_sent:
        return
    incident.notification_sent = True
    await db.flush()
    try:
        await notify_down(db, monitor, incident)
    except Exception:  # alert delivery must not roll back the incident
        logger.exception("Down notification failed")


async def _record_recovery(
    db: AsyncSession,
    monitor: Monitor,
    open_incident: Incident | None,
) -> None:
    if open_incident is None:
        return
    now = datetime.now(UTC)
    open_incident.resolved_at = now
    open_incident.duration_seconds = int((now - ensure_utc(open_incident.started_at)).total_seconds())
    await db.flush()
    if not monitor.alerts_enabled or open_incident.recovery_sent:
        return
    open_incident.recovery_sent = True
    await db.flush()
    try:
        await notify_recovery(db, monitor, open_incident)
    except Exception:  # alert delivery must not roll back the incident
        logger.exception("Recovery notification failed")


async def _get_open_incident(db: AsyncSession, monitor_id: int) -> Incident | None:
    result = await db.execute(
        select(Incident).where(
            Incident.monitor_id == monitor_id,
            Incident.resolved_at == None,
        )
    )
    return result.scalar_one_or_none()
