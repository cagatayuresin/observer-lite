from fastapi import APIRouter, Query
from jose import JWTError
from sse_starlette.sse import EventSourceResponse

from app.services.auth_service import decode_token
from app.sse.broadcaster import broadcaster

router = APIRouter(prefix="/api/sse", tags=["sse"])


@router.get("/dashboard")
async def sse_dashboard(token: str = Query(...)):
    """SSE endpoint — accepts JWT via query param (EventSource cannot set headers)."""
    try:
        payload = decode_token(token)
        if payload.get("type") != "access":
            raise ValueError()
    except (JWTError, ValueError):
        from fastapi import HTTPException
        raise HTTPException(status_code=401, detail="Invalid token")

    async def event_generator():
        async for event in broadcaster.subscribe():
            yield event

    return EventSourceResponse(event_generator())
