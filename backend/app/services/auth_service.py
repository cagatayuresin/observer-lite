"""Password hashing, JWT creation, and token decoding utilities.

All JWT operations use the ``HS256`` algorithm by default, configurable via
:attr:`~app.config.Settings.jwt_algorithm`.  Callers that need to handle
invalid tokens should catch :class:`jose.JWTError`.
"""

from datetime import datetime, timedelta, timezone

import bcrypt
from jose import jwt

from app.config import get_settings

settings = get_settings()

ALGORITHM = settings.jwt_algorithm


def hash_password(password: str) -> str:
    """Return a bcrypt hash of *password* with a freshly generated salt.

    Args:
        password: The plaintext password to hash.

    Returns:
        UTF-8 bcrypt hash string suitable for storing in the database.
    """
    return bcrypt.hashpw(password.encode(), bcrypt.gensalt()).decode()


def verify_password(password: str, password_hash: str) -> bool:
    """Check *password* against a previously computed *password_hash*.

    Args:
        password: Plaintext candidate password.
        password_hash: bcrypt hash stored in the database.

    Returns:
        ``True`` if the password matches, ``False`` otherwise.
    """
    return bcrypt.checkpw(password.encode(), password_hash.encode())


def create_access_token(user_id: int, role: str) -> str:
    """Create a short-lived JWT access token.

    Args:
        user_id: Database primary key of the user.
        role: User role string (``"superadmin"``, ``"admin"``, or
            ``"viewer"``).

    Returns:
        Signed JWT string.
    """
    expire = datetime.now(timezone.utc) + timedelta(minutes=settings.access_token_expire_minutes)
    payload = {"sub": str(user_id), "role": role, "type": "access", "exp": expire}
    return jwt.encode(payload, settings.secret_key, algorithm=ALGORITHM)


def create_refresh_token(user_id: int) -> str:
    """Create a long-lived JWT refresh token.

    The token contains only ``sub`` and ``type`` claims (no role) and is
    valid for :attr:`~app.config.Settings.refresh_token_expire_days` days.

    Args:
        user_id: Database primary key of the user.

    Returns:
        Signed JWT string.
    """
    expire = datetime.now(timezone.utc) + timedelta(days=settings.refresh_token_expire_days)
    payload = {"sub": str(user_id), "type": "refresh", "exp": expire}
    return jwt.encode(payload, settings.secret_key, algorithm=ALGORITHM)


def decode_token(token: str) -> dict:
    """Decode and verify a JWT produced by this application.

    Args:
        token: Encoded JWT string.

    Returns:
        The decoded claims dictionary.

    Raises:
        jose.JWTError: If the token signature is invalid, the token is
            expired, or it cannot be decoded.
    """
    return jwt.decode(token, settings.secret_key, algorithms=[ALGORITHM])
