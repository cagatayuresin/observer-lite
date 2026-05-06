"""Notification channel CRUD and test-send endpoints."""

import json
from datetime import datetime, timezone

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


@router.get("", response_model=list[ChannelOut])
async def list_channels(_: User = Depends(require_admin), db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(NotificationChannel).order_by(NotificationChannel.name))
    return result.scalars().all()


@router.post("", response_model=ChannelOut, status_code=201)
async def create_channel(body: ChannelCreate, current_user: User = Depends(require_admin), db: AsyncSession = Depends(get_db)):
    now = datetime.now(timezone.utc)
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


@router.get("/{channel_id}", response_model=ChannelOut)
async def get_channel(channel_id: int, _: User = Depends(require_admin), db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(NotificationChannel).where(NotificationChannel.id == channel_id))
    channel = result.scalar_one_or_none()
    if not channel:
        raise HTTPException(404, "Channel not found")
    return channel


@router.put("/{channel_id}", response_model=ChannelOut)
async def update_channel(channel_id: int, body: ChannelUpdate, _: User = Depends(require_admin), db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(NotificationChannel).where(NotificationChannel.id == channel_id))
    channel = result.scalar_one_or_none()
    if not channel:
        raise HTTPException(404, "Channel not found")
    if body.name is not None:
        channel.name = body.name
    if body.config is not None:
        channel.config = json.dumps(body.config)
    if body.is_enabled is not None:
        channel.is_enabled = body.is_enabled
    channel.updated_at = datetime.now(timezone.utc)
    await db.commit()
    await db.refresh(channel)
    return channel


@router.delete("/{channel_id}", status_code=204)
async def delete_channel(channel_id: int, _: User = Depends(require_admin), db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(NotificationChannel).where(NotificationChannel.id == channel_id))
    channel = result.scalar_one_or_none()
    if not channel:
        raise HTTPException(404, "Channel not found")
    await db.delete(channel)
    await db.commit()


@router.post("/{channel_id}/test")
async def test_channel(channel_id: int, _: User = Depends(require_admin), db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(NotificationChannel).where(NotificationChannel.id == channel_id))
    channel = result.scalar_one_or_none()
    if not channel:
        raise HTTPException(404, "Channel not found")

    if channel.channel_type == "email":
        ok = await test_email_channel(channel.config)
    elif channel.channel_type == "telegram":
        ok = await test_telegram_channel(channel.config)
    else:
        raise HTTPException(400, "Unknown channel type")

    if not ok:
        raise HTTPException(502, "Test notification failed — check channel configuration")
    return {"message": "Test notification sent successfully"}
