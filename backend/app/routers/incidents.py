"""Incident list, detail, and acknowledgement endpoints."""

from datetime import datetime, timezone

from fastapi import APIRouter, Depends, HTTPException, Query
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db.models import Incident, User
from app.db.session import get_db
from app.dependencies import get_current_user, require_admin
from app.schemas.incident import IncidentOut

router = APIRouter(prefix="/api/incidents", tags=["incidents"])


@router.get("", response_model=list[IncidentOut])
async def list_incidents(
    monitor_id: int | None = None,
    open_only: bool = False,
    limit: int = Query(100, le=500),
    _: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    q = select(Incident).order_by(Incident.started_at.desc()).limit(limit)
    if monitor_id:
        q = q.where(Incident.monitor_id == monitor_id)
    if open_only:
        q = q.where(Incident.resolved_at.is_(None))
    result = await db.execute(q)
    return result.scalars().all()


@router.get("/{incident_id}", response_model=IncidentOut)
async def get_incident(incident_id: int, _: User = Depends(get_current_user), db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(Incident).where(Incident.id == incident_id))
    inc = result.scalar_one_or_none()
    if not inc:
        raise HTTPException(404, "Incident not found")
    return inc


@router.post("/{incident_id}/acknowledge", status_code=204)
async def acknowledge(incident_id: int, current_user: User = Depends(require_admin), db: AsyncSession = Depends(get_db)):
    result2 = await db.execute(select(Incident).where(Incident.id == incident_id))
    inc = result2.scalar_one_or_none()
    if not inc:
        raise HTTPException(404, "Incident not found")
    inc.acknowledged_by = current_user.id
    inc.acknowledged_at = datetime.now(timezone.utc)
    await db.commit()
