"""Monitor CRUD, lifecycle control, bulk operations, and assignment endpoints."""

from datetime import datetime, timezone
import secrets

from fastapi import APIRouter, Depends, HTTPException, Query
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db.models import Monitor, MonitorUser, MonitorNotificationChannel, User
from app.db.session import get_db
from app.dependencies import get_current_user, require_admin
from app.schemas.monitor import MonitorBulkAction, MonitorCreate, MonitorOut, MonitorPatch, MonitorUpdate
from app.schemas.notification import MonitorChannelAssign
from app.scheduler.manager import pause_monitor_job, remove_monitor_job, upsert_monitor_job
from app.scheduler.jobs import run_monitor_check

router = APIRouter(prefix="/api/monitors", tags=["monitors"])


def _now() -> datetime:
    return datetime.now(timezone.utc)


@router.get("", response_model=list[MonitorOut])
async def list_monitors(
    status_filter: str | None = Query(None, alias="status"),
    group_id: int | None = None,
    search: str | None = None,
    current_user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    q = select(Monitor)
    if status_filter:
        q = q.where(Monitor.current_status == status_filter)
    if group_id:
        q = q.where(Monitor.group_id == group_id)
    if search:
        q = q.where(Monitor.name.ilike(f"%{search}%"))
    q = q.order_by(Monitor.name)
    result = await db.execute(q)
    return result.scalars().all()


@router.post("", response_model=MonitorOut, status_code=201)
async def create_monitor(
    body: MonitorCreate,
    current_user: User = Depends(require_admin),
    db: AsyncSession = Depends(get_db),
):
    now = _now()
    heartbeat_token = secrets.token_hex(16) if body.monitor_type == "heartbeat" else None
    monitor = Monitor(**body.model_dump(exclude={"heartbeat_grace_seconds"}), heartbeat_token=heartbeat_token, created_by=current_user.id, created_at=now, updated_at=now)
    monitor.heartbeat_grace_seconds = body.heartbeat_grace_seconds
    db.add(monitor)
    await db.commit()
    await db.refresh(monitor)

    if monitor.is_enabled:
        upsert_monitor_job(monitor)

    return monitor


@router.get("/{monitor_id}", response_model=MonitorOut)
async def get_monitor(monitor_id: int, _: User = Depends(get_current_user), db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(Monitor).where(Monitor.id == monitor_id))
    monitor = result.scalar_one_or_none()
    if not monitor:
        raise HTTPException(status_code=404, detail="Monitor not found")
    return monitor


@router.put("/{monitor_id}", response_model=MonitorOut)
async def update_monitor(
    monitor_id: int,
    body: MonitorUpdate,
    _: User = Depends(require_admin),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(Monitor).where(Monitor.id == monitor_id))
    monitor = result.scalar_one_or_none()
    if not monitor:
        raise HTTPException(status_code=404, detail="Monitor not found")

    update_data = body.model_dump(exclude_none=True)
    for key, value in update_data.items():
        setattr(monitor, key, value)
    monitor.updated_at = _now()
    await db.commit()
    await db.refresh(monitor)

    if monitor.is_enabled:
        upsert_monitor_job(monitor)
    else:
        pause_monitor_job(monitor.id)

    return monitor


@router.patch("/{monitor_id}", response_model=MonitorOut)
async def patch_monitor(
    monitor_id: int,
    body: MonitorPatch,
    _: User = Depends(require_admin),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(Monitor).where(Monitor.id == monitor_id))
    monitor = result.scalar_one_or_none()
    if not monitor:
        raise HTTPException(status_code=404, detail="Monitor not found")

    if body.is_enabled is not None:
        monitor.is_enabled = body.is_enabled
    if body.alerts_enabled is not None:
        monitor.alerts_enabled = body.alerts_enabled
    if body.ssl_check_enabled is not None:
        monitor.ssl_check_enabled = body.ssl_check_enabled
    monitor.updated_at = _now()
    await db.commit()
    await db.refresh(monitor)

    if monitor.is_enabled:
        upsert_monitor_job(monitor)
    else:
        pause_monitor_job(monitor.id)

    return monitor


@router.delete("/{monitor_id}", status_code=204)
async def delete_monitor(monitor_id: int, _: User = Depends(require_admin), db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(Monitor).where(Monitor.id == monitor_id))
    monitor = result.scalar_one_or_none()
    if not monitor:
        raise HTTPException(status_code=404, detail="Monitor not found")
    remove_monitor_job(monitor.id)
    await db.delete(monitor)
    await db.commit()


@router.post("/{monitor_id}/pause", status_code=204)
async def pause_monitor(monitor_id: int, _: User = Depends(require_admin), db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(Monitor).where(Monitor.id == monitor_id))
    monitor = result.scalar_one_or_none()
    if not monitor:
        raise HTTPException(status_code=404, detail="Monitor not found")
    monitor.is_enabled = False
    monitor.current_status = "paused"
    monitor.updated_at = _now()
    await db.commit()
    pause_monitor_job(monitor_id)


@router.post("/{monitor_id}/resume", status_code=204)
async def resume_monitor(monitor_id: int, _: User = Depends(require_admin), db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(Monitor).where(Monitor.id == monitor_id))
    monitor = result.scalar_one_or_none()
    if not monitor:
        raise HTTPException(status_code=404, detail="Monitor not found")
    monitor.is_enabled = True
    monitor.current_status = "pending"
    monitor.updated_at = _now()
    await db.commit()
    upsert_monitor_job(monitor)


@router.post("/{monitor_id}/check-now", status_code=202)
async def check_now(monitor_id: int, _: User = Depends(require_admin), db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(Monitor).where(Monitor.id == monitor_id))
    monitor = result.scalar_one_or_none()
    if not monitor:
        raise HTTPException(status_code=404, detail="Monitor not found")
    import asyncio
    asyncio.create_task(run_monitor_check(monitor_id))
    return {"message": "Check triggered"}


@router.post("/bulk", status_code=204)
async def bulk_action(
    body: MonitorBulkAction,
    _: User = Depends(require_admin),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(Monitor).where(Monitor.id.in_(body.ids)))
    monitors = result.scalars().all()
    for monitor in monitors:
        if body.action == "enable":
            monitor.is_enabled = True
            monitor.updated_at = _now()
            upsert_monitor_job(monitor)
        elif body.action == "disable":
            monitor.is_enabled = False
            monitor.updated_at = _now()
            pause_monitor_job(monitor.id)
        elif body.action == "delete":
            remove_monitor_job(monitor.id)
            await db.delete(monitor)
    await db.commit()


# --- User assignments ---
@router.get("/{monitor_id}/users")
async def get_monitor_users(monitor_id: int, _: User = Depends(require_admin), db: AsyncSession = Depends(get_db)):
    result = await db.execute(
        select(MonitorUser).where(MonitorUser.monitor_id == monitor_id)
    )
    return [{"user_id": mu.user_id, "notify": mu.notify} for mu in result.scalars().all()]


@router.post("/{monitor_id}/users", status_code=201)
async def assign_user(
    monitor_id: int,
    user_id: int,
    notify: bool = True,
    _: User = Depends(require_admin),
    db: AsyncSession = Depends(get_db),
):
    existing = await db.execute(
        select(MonitorUser).where(MonitorUser.monitor_id == monitor_id, MonitorUser.user_id == user_id)
    )
    if existing.scalar_one_or_none():
        raise HTTPException(status_code=409, detail="User already assigned")
    db.add(MonitorUser(monitor_id=monitor_id, user_id=user_id, notify=notify))
    await db.commit()
    return {"monitor_id": monitor_id, "user_id": user_id}


@router.delete("/{monitor_id}/users/{user_id}", status_code=204)
async def remove_user(monitor_id: int, user_id: int, _: User = Depends(require_admin), db: AsyncSession = Depends(get_db)):
    result = await db.execute(
        select(MonitorUser).where(MonitorUser.monitor_id == monitor_id, MonitorUser.user_id == user_id)
    )
    mu = result.scalar_one_or_none()
    if mu:
        await db.delete(mu)
        await db.commit()


# --- Notification channel assignments ---
@router.get("/{monitor_id}/channels")
async def get_monitor_channels(monitor_id: int, _: User = Depends(require_admin), db: AsyncSession = Depends(get_db)):
    result = await db.execute(
        select(MonitorNotificationChannel).where(MonitorNotificationChannel.monitor_id == monitor_id)
    )
    rows = result.scalars().all()
    return [{"channel_id": r.channel_id, "on_down": r.on_down, "on_recovery": r.on_recovery, "on_ssl_warn": r.on_ssl_warn} for r in rows]


@router.post("/{monitor_id}/channels", status_code=201)
async def assign_channel(
    monitor_id: int,
    body: MonitorChannelAssign,
    _: User = Depends(require_admin),
    db: AsyncSession = Depends(get_db),
):
    existing = await db.execute(
        select(MonitorNotificationChannel).where(
            MonitorNotificationChannel.monitor_id == monitor_id,
            MonitorNotificationChannel.channel_id == body.channel_id,
        )
    )
    if existing.scalar_one_or_none():
        raise HTTPException(status_code=409, detail="Channel already assigned")
    db.add(MonitorNotificationChannel(
        monitor_id=monitor_id,
        channel_id=body.channel_id,
        on_down=body.on_down,
        on_recovery=body.on_recovery,
        on_ssl_warn=body.on_ssl_warn,
    ))
    await db.commit()
    return {"monitor_id": monitor_id, "channel_id": body.channel_id}


@router.delete("/{monitor_id}/channels/{channel_id}", status_code=204)
async def remove_channel(monitor_id: int, channel_id: int, _: User = Depends(require_admin), db: AsyncSession = Depends(get_db)):
    result = await db.execute(
        select(MonitorNotificationChannel).where(
            MonitorNotificationChannel.monitor_id == monitor_id,
            MonitorNotificationChannel.channel_id == channel_id,
        )
    )
    mnc = result.scalar_one_or_none()
    if mnc:
        await db.delete(mnc)
        await db.commit()
