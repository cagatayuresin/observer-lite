"""Application settings read/write and SMTP test endpoints (superadmin only).

Settings are stored as key-value pairs in the ``app_settings`` table.
Sensitive values (e.g. ``smtp_password_enc``) are masked in GET responses.
"""

from datetime import datetime, timezone

from fastapi import APIRouter, Depends
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db.models import AppSetting, User
from app.db.session import get_db
from app.dependencies import require_superadmin

router = APIRouter(prefix="/api/settings", tags=["settings"])

_PUBLIC_KEYS = {"data_retention_days", "app_base_url"}
_SENSITIVE_KEYS = {"smtp_password_enc"}


@router.get("")
async def get_settings(_: User = Depends(require_superadmin), db: AsyncSession = Depends(get_db)):
    """Return all application settings with sensitive values masked."""
    result = await db.execute(select(AppSetting))
    settings = result.scalars().all()
    # Settings are intentionally schemaless for now, but secrets must never be
    # echoed back to the browser once saved.
    return {s.key: ("***" if s.key in _SENSITIVE_KEYS else s.value) for s in settings}


@router.put("")
async def update_settings(
    body: dict,
    current_user: User = Depends(require_superadmin),
    db: AsyncSession = Depends(get_db),
):
    """Create or update application settings from a partial key-value payload."""
    now = datetime.now(timezone.utc)
    for key, value in body.items():
        result = await db.execute(select(AppSetting).where(AppSetting.key == key))
        setting = result.scalar_one_or_none()
        if setting:
            setting.value = str(value)
            setting.updated_at = now
            setting.updated_by = current_user.id
        else:
            db.add(AppSetting(key=key, value=str(value), updated_at=now, updated_by=current_user.id))
    await db.commit()
    return {"message": "Settings updated"}


@router.post("/test-smtp")
async def test_smtp(_: User = Depends(require_superadmin), db: AsyncSession = Depends(get_db)):
    """Send a synthetic email notification using the saved SMTP settings."""
    result = await db.execute(select(AppSetting).where(AppSetting.key.in_(["smtp_host", "smtp_port", "smtp_user", "smtp_password_enc", "smtp_from"])))
    settings_rows = result.scalars().all()
    cfg = {s.key: s.value for s in settings_rows}
    # Import lazily so normal settings reads do not pull in SMTP dependencies.
    import json  # noqa: PLC0415 — deferred to keep top-level imports clean
    from app.services.email_service import test_email_channel
    ok = await test_email_channel(json.dumps({
        "smtp_host": cfg.get("smtp_host", ""),
        "smtp_port": cfg.get("smtp_port", 587),
        "smtp_user": cfg.get("smtp_user", ""),
        "smtp_password_enc": cfg.get("smtp_password_enc", ""),
        "smtp_from": cfg.get("smtp_from", ""),
        "recipients": [cfg.get("smtp_from", "")],
    }))
    if not ok:
        from fastapi import HTTPException  # noqa: PLC0415
        raise HTTPException(502, "SMTP test failed — check settings")
    return {"message": "SMTP test successful"}
