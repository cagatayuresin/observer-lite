from datetime import datetime

from fastapi import APIRouter, Depends, Query
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db.models import AuditLog, User
from app.db.session import get_db
from app.dependencies import require_superadmin
from pydantic import BaseModel


class AuditLogOut(BaseModel):
    id: int
    user_id: int | None
    action: str
    resource_type: str | None
    resource_id: int | None
    detail: str | None
    ip_address: str | None
    created_at: datetime
    model_config = {"from_attributes": True}


router = APIRouter(prefix="/api/audit-log", tags=["audit-log"])


@router.get("", response_model=list[AuditLogOut])
async def list_audit_log(
    action: str | None = None,
    user_id: int | None = None,
    limit: int = Query(200, le=1000),
    _: User = Depends(require_superadmin),
    db: AsyncSession = Depends(get_db),
):
    q = select(AuditLog).order_by(AuditLog.created_at.desc()).limit(limit)
    if action:
        q = q.where(AuditLog.action == action)
    if user_id:
        q = q.where(AuditLog.user_id == user_id)
    result = await db.execute(q)
    return result.scalars().all()
