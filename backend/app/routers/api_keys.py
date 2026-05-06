from datetime import datetime, timezone

from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db.models import ApiKey, User
from app.db.session import get_db
from app.dependencies import get_current_user
from app.schemas.api_key import ApiKeyCreate, ApiKeyCreated, ApiKeyOut
from app.utils.crypto import generate_api_key

router = APIRouter(prefix="/api/api-keys", tags=["api-keys"])


@router.get("", response_model=list[ApiKeyOut])
async def list_keys(current_user: User = Depends(get_current_user), db: AsyncSession = Depends(get_db)):
    q = select(ApiKey).where(ApiKey.user_id == current_user.id).order_by(ApiKey.created_at.desc())
    if current_user.role == "superadmin":
        q = select(ApiKey).order_by(ApiKey.created_at.desc())
    result = await db.execute(q)
    return result.scalars().all()


@router.post("", response_model=ApiKeyCreated, status_code=201)
async def create_key(body: ApiKeyCreate, current_user: User = Depends(get_current_user), db: AsyncSession = Depends(get_db)):
    raw_key, key_hash, key_prefix = generate_api_key()
    now = datetime.now(timezone.utc)
    api_key = ApiKey(
        user_id=current_user.id,
        name=body.name,
        key_hash=key_hash,
        key_prefix=key_prefix,
        expires_at=body.expires_at,
        created_at=now,
    )
    db.add(api_key)
    await db.commit()
    await db.refresh(api_key)
    return ApiKeyCreated(
        id=api_key.id,
        name=api_key.name,
        key_prefix=api_key.key_prefix,
        expires_at=api_key.expires_at,
        last_used_at=api_key.last_used_at,
        created_at=api_key.created_at,
        raw_key=raw_key,
    )


@router.delete("/{key_id}", status_code=204)
async def delete_key(key_id: int, current_user: User = Depends(get_current_user), db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(ApiKey).where(ApiKey.id == key_id))
    key = result.scalar_one_or_none()
    if not key:
        raise HTTPException(404, "API key not found")
    if key.user_id != current_user.id and current_user.role != "superadmin":
        raise HTTPException(403, "Cannot delete another user's API key")
    await db.delete(key)
    await db.commit()
