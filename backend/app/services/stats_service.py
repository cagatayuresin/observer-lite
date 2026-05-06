"""Monitor statistics aggregation.

Computes uptime percentage, average response time, and incident counts over a
configurable time window by running a handful of aggregate SQL queries.
Results are returned as a :class:`~app.schemas.stats.MonitorStats` Pydantic
model for direct serialisation by the stats router.
"""

from datetime import datetime, timedelta, timezone

from sqlalchemy import func, select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db.models import CheckResult, Incident
from app.schemas.stats import MonitorStats


async def get_monitor_stats(db: AsyncSession, monitor_id: int, days: int = 30) -> MonitorStats:
    """Aggregate check and incident data for *monitor_id* over the last *days*.

    Uptime is calculated as ``up_checks / total_checks * 100``.  Both ``"up"``
    and ``"warning"`` results count as "up" for this purpose.  If there are no
    checks in the window, uptime defaults to ``100.0`` (benefit of the doubt).

    Args:
        db: Async database session.
        monitor_id: Primary key of the target monitor.
        days: Look-back window in days (default 30).

    Returns:
        A populated :class:`~app.schemas.stats.MonitorStats` instance.
    """
    since = datetime.now(timezone.utc) - timedelta(days=days)

    # Total checks
    total_result = await db.execute(
        select(func.count()).where(
            CheckResult.monitor_id == monitor_id,
            CheckResult.checked_at >= since,
        )
    )
    total_checks = total_result.scalar_one() or 0

    # Up checks
    up_result = await db.execute(
        select(func.count()).where(
            CheckResult.monitor_id == monitor_id,
            CheckResult.checked_at >= since,
            CheckResult.status.in_(["up", "warning"]),
        )
    )
    up_checks = up_result.scalar_one() or 0

    uptime_pct = (up_checks / total_checks * 100) if total_checks > 0 else 100.0

    # Avg response time
    avg_result = await db.execute(
        select(func.avg(CheckResult.response_time_ms)).where(
            CheckResult.monitor_id == monitor_id,
            CheckResult.checked_at >= since,
            CheckResult.response_time_ms.is_not(None),
        )
    )
    avg_rt = avg_result.scalar_one()

    # Incidents
    total_inc_result = await db.execute(
        select(func.count()).where(
            Incident.monitor_id == monitor_id,
            Incident.started_at >= since,
        )
    )
    total_incidents = total_inc_result.scalar_one() or 0

    open_inc_result = await db.execute(
        select(func.count()).where(
            Incident.monitor_id == monitor_id,
            Incident.resolved_at.is_(None),
        )
    )
    open_incidents = open_inc_result.scalar_one() or 0

    return MonitorStats(
        monitor_id=monitor_id,
        uptime_percent=round(uptime_pct, 3),
        avg_response_time_ms=round(float(avg_rt), 1) if avg_rt is not None else None,
        total_checks=total_checks,
        total_incidents=total_incidents,
        open_incidents=open_incidents,
        period_days=days,
    )
