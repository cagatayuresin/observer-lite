from datetime import datetime, timezone

from fastapi import APIRouter, HTTPException
from sqlalchemy import select

from app.db.models import Monitor
from app.db.session import AsyncSessionLocal

router = APIRouter(prefix="/api/heartbeat", tags=["heartbeat"])


async def _handle_heartbeat(token: str):
    async with AsyncSessionLocal() as db:
        result = await db.execute(select(Monitor).where(Monitor.heartbeat_token == token))
        monitor = result.scalar_one_or_none()
        if not monitor:
            raise HTTPException(404, "Unknown heartbeat token")
        monitor.heartbeat_last_ping = datetime.now(timezone.utc)
        monitor.current_status = "up"
        monitor.last_checked_at = monitor.heartbeat_last_ping
        monitor.consecutive_failures = 0
        monitor.updated_at = datetime.now(timezone.utc)
        await db.commit()
        return {"message": "ok", "monitor": monitor.name}


@router.get("/{token}")
async def heartbeat_get(token: str):
    return await _handle_heartbeat(token)


@router.post("/{token}")
async def heartbeat_post(token: str):
    return await _handle_heartbeat(token)
