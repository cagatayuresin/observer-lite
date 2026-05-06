"""Public heartbeat ingest endpoints.

Heartbeat monitors are passive: the watched service calls one of these
endpoints to prove it is still alive.  A successful ping updates the monitor's
last-seen timestamp and clears transient failure counters so the scheduler can
detect silence later.
"""

from datetime import datetime, timezone

from fastapi import APIRouter, HTTPException
from sqlalchemy import select

from app.db.models import Monitor
from app.db.session import AsyncSessionLocal

router = APIRouter(prefix="/api/heartbeat", tags=["heartbeat"])


async def _handle_heartbeat(token: str):
    """Record a heartbeat ping for the monitor that owns *token*.

    The endpoint deliberately does not require user authentication.  The
    randomly generated heartbeat token is the shared secret, which keeps
    external cron jobs and small services easy to integrate.
    """
    # This route is also called by unauthenticated clients, so it bypasses the
    # request-scoped get_db dependency and opens a short, self-contained session.
    async with AsyncSessionLocal() as db:
        result = await db.execute(select(Monitor).where(Monitor.heartbeat_token == token))
        monitor = result.scalar_one_or_none()
        if not monitor:
            raise HTTPException(404, "Unknown heartbeat token")
        # A heartbeat is an explicit "I am alive" signal, so it resets the
        # live status immediately instead of waiting for the next scheduler run.
        monitor.heartbeat_last_ping = datetime.now(timezone.utc)
        monitor.current_status = "up"
        monitor.last_checked_at = monitor.heartbeat_last_ping
        monitor.consecutive_failures = 0
        monitor.updated_at = datetime.now(timezone.utc)
        await db.commit()
        return {"message": "ok", "monitor": monitor.name}


@router.get("/{token}")
async def heartbeat_get(token: str):
    """Accept a heartbeat ping sent as an HTTP GET request."""
    return await _handle_heartbeat(token)


@router.post("/{token}")
async def heartbeat_post(token: str):
    """Accept a heartbeat ping sent as an HTTP POST request."""
    return await _handle_heartbeat(token)
