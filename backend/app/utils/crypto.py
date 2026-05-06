"""Cryptographic utilities: symmetric encryption, API key generation, and hashing.

All encryption uses **Fernet** (AES-128-CBC + HMAC-SHA256).  The Fernet key
is derived from :attr:`~app.config.Settings.secret_key` via PBKDF2-HMAC-SHA256
(100 000 iterations) so that changing ``SECRET_KEY`` invalidates all stored
ciphertext — a deliberate design choice that forces re-entry of credentials
after a key rotation.

API keys are 256-bit random values prefixed with ``obs_``.  The raw key is
returned once at creation time; only its SHA-256 hash is stored in the DB.
"""

import base64
import hashlib
import os

from cryptography.fernet import Fernet
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2HMAC

from app.config import get_settings


def _get_fernet() -> Fernet:
    """Derive a Fernet instance from the application secret key.

    The same salt is used every time so that the derived key is stable for a
    given ``SECRET_KEY`` value.  A new ``SECRET_KEY`` means a new Fernet key,
    which makes all previously encrypted values unreadable.

    Returns:
        A ready-to-use :class:`~cryptography.fernet.Fernet` instance.
    """
    settings = get_settings()
    kdf = PBKDF2HMAC(
        algorithm=hashes.SHA256(),
        length=32,
        salt=b"observer-lite-salt",
        iterations=100_000,
    )
    key = base64.urlsafe_b64encode(kdf.derive(settings.secret_key.encode()))
    return Fernet(key)


def encrypt(plaintext: str) -> str:
    """Encrypt *plaintext* with Fernet and return a URL-safe base64 string.

    Args:
        plaintext: The secret value to protect (e.g. an SMTP password).

    Returns:
        Fernet token as a UTF-8 string.
    """
    return _get_fernet().encrypt(plaintext.encode()).decode()


def decrypt(ciphertext: str) -> str:
    """Decrypt a Fernet token produced by :func:`encrypt`.

    Args:
        ciphertext: Fernet token returned by :func:`encrypt`.

    Returns:
        The original plaintext string.

    Raises:
        cryptography.fernet.InvalidToken: If the token is invalid or was
            encrypted with a different key.
    """
    return _get_fernet().decrypt(ciphertext.encode()).decode()


def generate_api_key() -> tuple[str, str, str]:
    """Generate a new API key.

    Returns:
        A 3-tuple ``(raw_key, key_hash, key_prefix)`` where *raw_key* is the
        full ``obs_``-prefixed key shown to the user exactly once, *key_hash*
        is its SHA-256 hex digest stored in the database, and *key_prefix* is
        the first 10 characters used for display purposes.
    """
    raw = "obs_" + os.urandom(28).hex()
    key_hash = hashlib.sha256(raw.encode()).hexdigest()
    key_prefix = raw[:10]
    return raw, key_hash, key_prefix


def hash_api_key(raw: str) -> str:
    """Return the SHA-256 hex digest of *raw*.

    Used both when storing a newly generated key and when looking up an
    incoming API key from the ``Authorization`` header.

    Args:
        raw: The full ``obs_``-prefixed API key string.

    Returns:
        64-character lowercase hex string.
    """
    return hashlib.sha256(raw.encode()).hexdigest()
