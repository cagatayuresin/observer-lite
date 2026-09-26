"""Public heartbeat ingest endpoints.

Heartbeat monitors are passive: the watched service calls one of these
endpoints to prove it is still alive.  A successful ping updates the monitor's
last-seen timestamp and clears transient failure counters so the scheduler can
detect silence later.
"""

from datetime import UTC, datetime

from fastapi import APIRouter, HTTPException
from sqlalchemy import select

from app.db.models import Monitor
from app.db.session import AsyncSessionLocal

router = APIRouter(prefix="/api/heartbeat", tags=["heartbeat"])

_UNKNOWN_TOKEN = "Unknown heartbeat token"


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
            return None
        # A heartbeat is an explicit "I am alive" signal, so it resets the
        # live status immediately instead of waiting for the next scheduler run.
        monitor.heartbeat_last_ping = datetime.now(UTC)
        monitor.current_status = "up"
        monitor.last_checked_at = monitor.heartbeat_last_ping
        monitor.consecutive_failures = 0
        monitor.updated_at = datetime.now(UTC)
        await db.commit()
        return {"message": "ok", "monitor": monitor.name}


@router.get("/{token}", responses={404: {"description": _UNKNOWN_TOKEN}})
async def heartbeat_get(token: str):
    """Accept a heartbeat ping sent as an HTTP GET request."""
    result = await _handle_heartbeat(token)
    if result is None:
        raise HTTPException(status_code=404, detail=_UNKNOWN_TOKEN)
    return result


@router.post("/{token}", responses={404: {"description": _UNKNOWN_TOKEN}})
async def heartbeat_post(token: str):
    """Accept a heartbeat ping sent as an HTTP POST request."""
    result = await _handle_heartbeat(token)
    if result is None:
        raise HTTPException(status_code=404, detail=_UNKNOWN_TOKEN)
    return result
