"""Notification channel CRUD and test-send endpoints."""

import json
from datetime import UTC, datetime

from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db.models import NotificationChannel, User
from app.db.session import get_db
from app.dependencies import require_admin
from app.schemas.notification import ChannelCreate, ChannelOut, ChannelUpdate
from app.services.email_service import test_email_channel
from app.services.telegram_service import test_telegram_channel

router = APIRouter(prefix="/api/channels", tags=["channels"])

_CHANNEL_NOT_FOUND = "Channel not found"


@router.get("", response_model=list[ChannelOut])
async def list_channels(_: User = Depends(require_admin), db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(NotificationChannel).order_by(NotificationChannel.name))
    return result.scalars().all()


@router.post("", response_model=ChannelOut, status_code=201)
async def create_channel(body: ChannelCreate, current_user: User = Depends(require_admin), db: AsyncSession = Depends(get_db)):
    now = datetime.now(UTC)
    channel = NotificationChannel(
        name=body.name,
        channel_type=body.channel_type,
        config=json.dumps(body.config),
        is_enabled=body.is_enabled,
        created_by=current_user.id,
        created_at=now,
        updated_at=now,
    )
    db.add(channel)
    await db.commit()
    await db.refresh(channel)
    return channel


@router.get("/{channel_id}", response_model=ChannelOut, responses={404: {"description": _CHANNEL_NOT_FOUND}})
async def get_channel(channel_id: int, _: User = Depends(require_admin), db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(NotificationChannel).where(NotificationChannel.id == channel_id))
    channel = result.scalar_one_or_none()
    if not channel:
        raise HTTPException(status_code=404, detail=_CHANNEL_NOT_FOUND)
    return channel


@router.put("/{channel_id}", response_model=ChannelOut, responses={404: {"description": _CHANNEL_NOT_FOUND}})
async def update_channel(channel_id: int, body: ChannelUpdate, _: User = Depends(require_admin), db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(NotificationChannel).where(NotificationChannel.id == channel_id))
    channel = result.scalar_one_or_none()
    if not channel:
        raise HTTPException(status_code=404, detail=_CHANNEL_NOT_FOUND)
    if body.name is not None:
        channel.name = body.name
    if body.config is not None:
        channel.config = json.dumps(body.config)
    if body.is_enabled is not None:
        channel.is_enabled = body.is_enabled
    channel.updated_at = datetime.now(UTC)
    await db.commit()
    await db.refresh(channel)
    return channel


@router.delete("/{channel_id}", status_code=204, responses={404: {"description": _CHANNEL_NOT_FOUND}})
async def delete_channel(channel_id: int, _: User = Depends(require_admin), db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(NotificationChannel).where(NotificationChannel.id == channel_id))
    channel = result.scalar_one_or_none()
    if not channel:
        raise HTTPException(status_code=404, detail=_CHANNEL_NOT_FOUND)
    await db.delete(channel)
    await db.commit()


@router.post(
    "/{channel_id}/test",
    responses={
        404: {"description": _CHANNEL_NOT_FOUND},
        400: {"description": "Unknown channel type"},
        502: {"description": "Test notification failed"},
    },
)
async def test_channel(channel_id: int, _: User = Depends(require_admin), db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(NotificationChannel).where(NotificationChannel.id == channel_id))
    channel = result.scalar_one_or_none()
    if not channel:
        raise HTTPException(status_code=404, detail=_CHANNEL_NOT_FOUND)

    if channel.channel_type == "email":
        ok = await test_email_channel(channel.config)
    elif channel.channel_type == "telegram":
        ok = await test_telegram_channel(channel.config)
    else:
        raise HTTPException(status_code=400, detail="Unknown channel type")

    if not ok:
        raise HTTPException(status_code=502, detail="Test notification failed — check channel configuration")
    return {"message": "Test notification sent successfully"}
