from fastapi import APIRouter, Depends, Query
from sqlalchemy.ext.asyncio import AsyncSession

from app.db.models import User
from app.db.session import get_db
from app.dependencies import get_current_user
from app.schemas.stats import MonitorStats
from app.services.stats_service import get_monitor_stats

router = APIRouter(prefix="/api/monitors", tags=["stats"])


@router.get("/{monitor_id}/stats", response_model=MonitorStats)
async def monitor_stats(
    monitor_id: int,
    days: int = Query(30, ge=1, le=365),
    _: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    return await get_monitor_stats(db, monitor_id, days)
