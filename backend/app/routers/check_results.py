from datetime import datetime

from fastapi import APIRouter, Depends, Query
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db.models import CheckResult, User
from app.db.session import get_db
from app.dependencies import get_current_user
from app.schemas.check_result import CheckResultOut

router = APIRouter(prefix="/api/check-results", tags=["check-results"])


@router.get("", response_model=list[CheckResultOut])
async def list_results(
    monitor_id: int | None = None,
    from_dt: datetime | None = Query(None, alias="from"),
    to_dt: datetime | None = Query(None, alias="to"),
    limit: int = Query(200, le=1000),
    _: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    q = select(CheckResult).order_by(CheckResult.checked_at.desc()).limit(limit)
    if monitor_id:
        q = q.where(CheckResult.monitor_id == monitor_id)
    if from_dt:
        q = q.where(CheckResult.checked_at >= from_dt)
    if to_dt:
        q = q.where(CheckResult.checked_at <= to_dt)
    result = await db.execute(q)
    return result.scalars().all()
