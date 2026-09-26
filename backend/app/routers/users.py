"""User management endpoints (superadmin only)."""

from datetime import UTC, datetime

from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db.models import User
from app.db.session import get_db
from app.dependencies import require_superadmin
from app.schemas.user import UserCreate, UserOut, UserUpdate
from app.services.auth_service import hash_password

router = APIRouter(prefix="/api/users", tags=["users"])

_USER_NOT_FOUND = "User not found"


@router.get("", response_model=list[UserOut])
async def list_users(
    _: User = Depends(require_superadmin),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(User).order_by(User.created_at.desc()))
    return result.scalars().all()


@router.post(
    "",
    response_model=UserOut,
    status_code=201,
    responses={409: {"description": "Username already exists"}},
)
async def create_user(
    body: UserCreate,
    _: User = Depends(require_superadmin),
    db: AsyncSession = Depends(get_db),
):
    existing = await db.execute(select(User).where(User.username == body.username))
    if existing.scalar_one_or_none():
        raise HTTPException(status_code=409, detail="Username already exists")

    now = datetime.now(UTC)
    user = User(
        username=body.username,
        email=body.email,
        password_hash=hash_password(body.password),
        role=body.role,
        force_pw_change=True,
        created_at=now,
        updated_at=now,
    )
    db.add(user)
    await db.commit()
    await db.refresh(user)
    return user


@router.get("/{user_id}", response_model=UserOut, responses={404: {"description": _USER_NOT_FOUND}})
async def get_user(
    user_id: int,
    _: User = Depends(require_superadmin),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(User).where(User.id == user_id))
    user = result.scalar_one_or_none()
    if not user:
        raise HTTPException(status_code=404, detail=_USER_NOT_FOUND)
    return user


@router.put("/{user_id}", response_model=UserOut, responses={404: {"description": _USER_NOT_FOUND}})
async def update_user(
    user_id: int,
    body: UserUpdate,
    _: User = Depends(require_superadmin),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(User).where(User.id == user_id))
    user = result.scalar_one_or_none()
    if not user:
        raise HTTPException(status_code=404, detail=_USER_NOT_FOUND)
    if body.email is not None:
        user.email = body.email
    if body.role is not None:
        user.role = body.role
    if body.is_active is not None:
        user.is_active = body.is_active
    user.updated_at = datetime.now(UTC)
    await db.commit()
    await db.refresh(user)
    return user


@router.delete(
    "/{user_id}",
    status_code=204,
    responses={
        400: {"description": "Cannot delete yourself"},
        404: {"description": _USER_NOT_FOUND},
    },
)
async def delete_user(
    user_id: int,
    current_user: User = Depends(require_superadmin),
    db: AsyncSession = Depends(get_db),
):
    if user_id == current_user.id:
        raise HTTPException(status_code=400, detail="Cannot delete yourself")
    result = await db.execute(select(User).where(User.id == user_id))
    user = result.scalar_one_or_none()
    if not user:
        raise HTTPException(status_code=404, detail=_USER_NOT_FOUND)
    await db.delete(user)
    await db.commit()


@router.post(
    "/{user_id}/reset-password",
    status_code=204,
    responses={404: {"description": _USER_NOT_FOUND}},
)
async def reset_password(
    user_id: int,
    _: User = Depends(require_superadmin),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(User).where(User.id == user_id))
    user = result.scalar_one_or_none()
    if not user:
        raise HTTPException(status_code=404, detail=_USER_NOT_FOUND)
    user.password_hash = hash_password("password123")
    user.force_pw_change = True
    user.updated_at = datetime.now(UTC)
    await db.commit()
