"""Application configuration loaded from environment variables or a .env file.

Exposes a single :func:`get_settings` factory (cached) so every module can
call ``get_settings()`` and receive the same :class:`Settings` instance.
"""

from functools import lru_cache

from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    """Pydantic-Settings model for all runtime configuration.

    Values are read from environment variables first, then from a ``.env``
    file in the working directory.  Unknown env vars are silently ignored.

    Attributes:
        app_name: Human-readable application name (shown in OpenAPI docs).
        secret_key: HMAC secret used for JWT signing — **must** be changed
            in production.
        debug: When ``True``, SQLAlchemy echoes all SQL to stdout.
        port: TCP port the Uvicorn server listens on.
        database_path: Absolute filesystem path to the SQLite file.
        jwt_algorithm: JWT signing algorithm (e.g. ``"HS256"``).
        access_token_expire_minutes: Lifetime of short-lived access tokens.
        refresh_token_expire_days: Lifetime of long-lived refresh tokens.
        rate_limit_login: SlowAPI rate-limit string for the login endpoint.
        rate_limit_heartbeat: SlowAPI rate-limit string for heartbeat pings.
        max_concurrent_checks: Maximum number of simultaneous outbound checks
            the scheduler is allowed to run.
    """

    model_config = SettingsConfigDict(env_file=".env", env_file_encoding="utf-8", extra="ignore")

    # App
    app_name: str = "Observer Lite"
    secret_key: str = "change-me-in-production-use-a-long-random-string"
    debug: bool = False
    port: int = 3000

    # Database
    database_path: str = "/data/observer.db"

    @property
    def database_url(self) -> str:
        """Return the async SQLAlchemy connection URL for the SQLite database."""
        return f"sqlite+aiosqlite:///{self.database_path}"

    # JWT
    jwt_algorithm: str = "HS256"
    access_token_expire_minutes: int = 15
    refresh_token_expire_days: int = 30

    # Rate limiting
    rate_limit_login: str = "5/minute"
    rate_limit_heartbeat: str = "1/10seconds"

    # Scheduler
    max_concurrent_checks: int = 200


@lru_cache
def get_settings() -> Settings:
    """Return the application-wide :class:`Settings` singleton.

    The result is cached after the first call so that ``pydantic-settings``
    only reads the environment / ``.env`` file once per process lifetime.

    Returns:
        A fully-populated :class:`Settings` instance.
    """
    return Settings()
