from datetime import datetime, timezone

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db.models import MonitorGroup, User
from app.db.session import get_db
from app.dependencies import get_current_user, require_admin

router = APIRouter(prefix="/api/groups", tags=["groups"])


class GroupCreate(BaseModel):
    name: str
    description: str | None = None


class GroupOut(BaseModel):
    id: int
    name: str
    description: str | None
    created_at: datetime
    model_config = {"from_attributes": True}


@router.get("", response_model=list[GroupOut])
async def list_groups(_: User = Depends(get_current_user), db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(MonitorGroup).order_by(MonitorGroup.name))
    return result.scalars().all()


@router.post("", response_model=GroupOut, status_code=201)
async def create_group(body: GroupCreate, current_user: User = Depends(require_admin), db: AsyncSession = Depends(get_db)):
    now = datetime.now(timezone.utc)
    group = MonitorGroup(name=body.name, description=body.description, created_by=current_user.id, created_at=now)
    db.add(group)
    await db.commit()
    await db.refresh(group)
    return group


@router.put("/{group_id}", response_model=GroupOut)
async def update_group(group_id: int, body: GroupCreate, _: User = Depends(require_admin), db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(MonitorGroup).where(MonitorGroup.id == group_id))
    group = result.scalar_one_or_none()
    if not group:
        raise HTTPException(404, "Group not found")
    group.name = body.name
    group.description = body.description
    await db.commit()
    await db.refresh(group)
    return group


@router.delete("/{group_id}", status_code=204)
async def delete_group(group_id: int, _: User = Depends(require_admin), db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(MonitorGroup).where(MonitorGroup.id == group_id))
    group = result.scalar_one_or_none()
    if not group:
        raise HTTPException(404, "Group not found")
    await db.delete(group)
    await db.commit()
