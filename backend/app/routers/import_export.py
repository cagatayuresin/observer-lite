"""Monitor import/export endpoints for portable open-source deployments.

Exports are intentionally limited to configuration fields that can be safely
re-created on another installation.  Runtime state such as current status,
heartbeat tokens, incidents, and check history is excluded.
"""

from datetime import datetime, timezone

from fastapi import APIRouter, Depends
from fastapi.responses import JSONResponse
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db.models import Monitor, User
from app.db.session import get_db
from app.dependencies import require_superadmin
from app.scheduler.manager import upsert_monitor_job

router = APIRouter(prefix="/api", tags=["import-export"])

# Keep this allowlist explicit so future model fields are not accidentally
# leaked or imported before their portability/security implications are known.
_EXPORT_FIELDS = [
    "name", "url", "monitor_type", "check_interval_seconds", "timeout_seconds",
    "retry_count", "retry_interval_seconds", "alert_cooldown_seconds",
    "request_headers", "request_body", "expected_status_codes",
    "expected_body_type", "expected_body_value", "response_time_warning_ms",
    "ssl_check_enabled", "ssl_expiry_warning_days", "alerts_enabled",
    "heartbeat_grace_seconds",
]


@router.get("/export/monitors")
async def export_monitors(_: User = Depends(require_superadmin), db: AsyncSession = Depends(get_db)):
    """Export monitor definitions as a stable JSON payload."""
    result = await db.execute(select(Monitor).order_by(Monitor.id))
    monitors = result.scalars().all()
    data = [{field: getattr(m, field) for field in _EXPORT_FIELDS} for m in monitors]
    return JSONResponse(content={"version": 1, "monitors": data, "exported_at": datetime.now(timezone.utc).isoformat()})


@router.post("/import/monitors")
async def import_monitors(
    body: dict,
    current_user: User = Depends(require_superadmin),
    db: AsyncSession = Depends(get_db),
):
    """Import monitor definitions from the JSON format produced by export."""
    monitors_data = body.get("monitors", [])
    created = 0
    now = datetime.now(timezone.utc)
    for m_data in monitors_data:
        # Ignore unknown keys to preserve forward compatibility between
        # versions and to avoid mass-assigning fields such as IDs or status.
        monitor = Monitor(
            **{k: v for k, v in m_data.items() if k in _EXPORT_FIELDS},
            created_by=current_user.id,
            created_at=now,
            updated_at=now,
        )
        db.add(monitor)
        await db.flush()
        if monitor.is_enabled:
            # Imported enabled monitors should begin checking immediately after
            # the transaction commits, matching newly created monitors.
            upsert_monitor_job(monitor)
        created += 1
    await db.commit()
    return {"imported": created}
