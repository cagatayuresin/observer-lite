"""FastAPI dependency functions for authentication and role-based access control.

This module provides:
- :func:`get_current_user` — resolves the authenticated user from either a
  JWT Bearer token or an ``obs_`` prefixed API key.
- :func:`require_role` — factory that builds a dependency enforcing one of
  the given roles.
- Pre-built shortcuts :data:`require_admin` and :data:`require_superadmin`.
"""

from fastapi import Depends, HTTPException, Security, status
from fastapi.security import HTTPAuthorizationCredentials, HTTPBearer
from jose import JWTError
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db.models import ApiKey, User
from app.db.session import get_db
from app.services.auth_service import decode_token
from app.utils.crypto import hash_api_key

bearer_scheme = HTTPBearer(auto_error=False)


async def get_current_user(
    credentials: HTTPAuthorizationCredentials | None = Security(bearer_scheme),
    db: AsyncSession = Depends(get_db),
) -> User:
    """Resolve the currently authenticated :class:`~app.db.models.User`.

    Supports two authentication schemes:

    1. **API key** — token starts with ``obs_``.  The raw key is hashed and
       looked up in the ``api_keys`` table; ``last_used_at`` is updated on
       each call.
    2. **JWT access token** — decoded with :func:`~app.services.auth_service.decode_token`;
       the ``type`` claim must equal ``"access"``.

    Args:
        credentials: Bearer token extracted by :data:`bearer_scheme`, or
            ``None`` if the ``Authorization`` header is absent.
        db: Async database session injected by :func:`~app.db.session.get_db`.

    Returns:
        The active :class:`~app.db.models.User` that owns the credential.

    Raises:
        HTTPException: 401 if the token is missing, invalid, expired, or
            belongs to an inactive / non-existent user.
    """
    credentials_exception = HTTPException(
        status_code=status.HTTP_401_UNAUTHORIZED,
        detail="Could not validate credentials",
        headers={"WWW-Authenticate": "Bearer"},
    )
    if credentials is None:
        raise credentials_exception

    token = credentials.credentials

    # Try API key first (prefix: obs_)
    if token.startswith("obs_"):
        key_hash = hash_api_key(token)
        result = await db.execute(
            select(ApiKey).where(ApiKey.key_hash == key_hash)
        )
        api_key = result.scalar_one_or_none()
        if api_key is None:
            raise credentials_exception
        # Update last_used
        from datetime import datetime, timezone
        api_key.last_used_at = datetime.now(timezone.utc)
        await db.commit()
        result = await db.execute(select(User).where(User.id == api_key.user_id))
        user = result.scalar_one_or_none()
    else:
        try:
            payload = decode_token(token)
            if payload.get("type") != "access":
                raise credentials_exception
            user_id = int(payload["sub"])
        except (JWTError, KeyError, ValueError):
            raise credentials_exception
        result = await db.execute(select(User).where(User.id == user_id))
        user = result.scalar_one_or_none()

    if user is None or not user.is_active:
        raise credentials_exception
    return user


def require_role(*roles: str):
    """Return a FastAPI dependency that enforces membership in one of *roles*.

    Args:
        *roles: Accepted role strings (e.g. ``"admin"``, ``"superadmin"``).

    Returns:
        An async callable suitable for use as a ``Depends`` argument.  It
        returns the authenticated user if their role is allowed, otherwise
        raises a 403 error.
    """
    async def _check(current_user: User = Depends(get_current_user)) -> User:
        if current_user.role not in roles:
            raise HTTPException(status_code=status.HTTP_403_FORBIDDEN, detail="Insufficient permissions")
        return current_user
    return _check


require_admin = require_role("admin", "superadmin")
require_superadmin = require_role("superadmin")
