from datetime import datetime, timezone

from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db.models import MaintenanceWindow, MaintenanceWindowMonitor, User
from app.db.session import get_db
from app.dependencies import get_current_user, require_admin
from app.schemas.maintenance import MaintenanceCreate, MaintenanceOut, MaintenanceUpdate

router = APIRouter(prefix="/api/maintenance", tags=["maintenance"])


@router.get("", response_model=list[MaintenanceOut])
async def list_windows(_: User = Depends(get_current_user), db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(MaintenanceWindow).order_by(MaintenanceWindow.starts_at.desc()))
    windows = result.scalars().all()
    return [await _to_out(db, w) for w in windows]


@router.post("", response_model=MaintenanceOut, status_code=201)
async def create_window(body: MaintenanceCreate, current_user: User = Depends(require_admin), db: AsyncSession = Depends(get_db)):
    now = datetime.now(timezone.utc)
    window = MaintenanceWindow(
        name=body.name,
        starts_at=body.starts_at,
        ends_at=body.ends_at,
        repeat_cron=body.repeat_cron,
        suppress_alerts=body.suppress_alerts,
        created_by=current_user.id,
        created_at=now,
    )
    db.add(window)
    await db.flush()
    for mid in body.monitor_ids:
        db.add(MaintenanceWindowMonitor(window_id=window.id, monitor_id=mid))
    await db.commit()
    await db.refresh(window)
    return await _to_out(db, window)


@router.get("/{window_id}", response_model=MaintenanceOut)
async def get_window(window_id: int, _: User = Depends(get_current_user), db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(MaintenanceWindow).where(MaintenanceWindow.id == window_id))
    window = result.scalar_one_or_none()
    if not window:
        raise HTTPException(404, "Maintenance window not found")
    return await _to_out(db, window)


@router.put("/{window_id}", response_model=MaintenanceOut)
async def update_window(window_id: int, body: MaintenanceUpdate, _: User = Depends(require_admin), db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(MaintenanceWindow).where(MaintenanceWindow.id == window_id))
    window = result.scalar_one_or_none()
    if not window:
        raise HTTPException(404, "Maintenance window not found")
    if body.name is not None:
        window.name = body.name
    if body.starts_at is not None:
        window.starts_at = body.starts_at
    if body.ends_at is not None:
        window.ends_at = body.ends_at
    if body.repeat_cron is not None:
        window.repeat_cron = body.repeat_cron
    if body.suppress_alerts is not None:
        window.suppress_alerts = body.suppress_alerts
    if body.monitor_ids is not None:
        await db.execute(
            select(MaintenanceWindowMonitor).where(MaintenanceWindowMonitor.window_id == window_id)
        )
        # Delete existing
        from sqlalchemy import delete as sa_delete
        await db.execute(sa_delete(MaintenanceWindowMonitor).where(MaintenanceWindowMonitor.window_id == window_id))
        for mid in body.monitor_ids:
            db.add(MaintenanceWindowMonitor(window_id=window_id, monitor_id=mid))
    await db.commit()
    await db.refresh(window)
    return await _to_out(db, window)


@router.delete("/{window_id}", status_code=204)
async def delete_window(window_id: int, _: User = Depends(require_admin), db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(MaintenanceWindow).where(MaintenanceWindow.id == window_id))
    window = result.scalar_one_or_none()
    if not window:
        raise HTTPException(404, "Maintenance window not found")
    await db.delete(window)
    await db.commit()


async def _to_out(db: AsyncSession, window: MaintenanceWindow) -> MaintenanceOut:
    result = await db.execute(
        select(MaintenanceWindowMonitor.monitor_id).where(MaintenanceWindowMonitor.window_id == window.id)
    )
    monitor_ids = [r[0] for r in result.all()]
    return MaintenanceOut(
        id=window.id,
        name=window.name,
        starts_at=window.starts_at,
        ends_at=window.ends_at,
        repeat_cron=window.repeat_cron,
        suppress_alerts=window.suppress_alerts,
        created_at=window.created_at,
        monitor_ids=monitor_ids,
    )
