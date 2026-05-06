"""Shared pytest fixtures for the Observer Lite test suite.

Uses an in-memory SQLite database for all tests.  The scheduler is disabled
so that no real monitor checks fire during tests.  Each test gets a fresh
database via the ``db`` fixture, which rolls back all changes after each test.
"""

import os
import pytest_asyncio
from datetime import datetime, timezone
from unittest.mock import patch

# Point to in-memory SQLite before any app code loads settings
os.environ.setdefault("DATABASE_PATH", ":memory:")
os.environ.setdefault("DATABASE_URL", "sqlite+aiosqlite:///:memory:")
os.environ.setdefault("SECRET_KEY", "test-secret-key-for-testing-only")
os.environ.setdefault("DEBUG", "false")

from httpx import ASGITransport, AsyncClient
from sqlalchemy.ext.asyncio import async_sessionmaker, create_async_engine

from app.db.session import Base
from app.db.models import User  # noqa: F401 — registers all models
from app.db.models import (  # noqa: F401
    ApiKey, Monitor, CheckResult, Incident,
    NotificationChannel, MonitorNotificationChannel,
    AppSetting, AuditLog, MonitorGroup, MonitorUser,
    MaintenanceWindow, MaintenanceWindowMonitor,
)
from app.services.auth_service import create_access_token, hash_password

TEST_DB_URL = "sqlite+aiosqlite:///:memory:"


@pytest_asyncio.fixture(scope="function")
async def engine():
    """Create a fresh in-memory engine for each test function."""
    eng = create_async_engine(TEST_DB_URL, echo=False)
    async with eng.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)
    yield eng
    async with eng.begin() as conn:
        await conn.run_sync(Base.metadata.drop_all)
    await eng.dispose()


@pytest_asyncio.fixture(scope="function")
async def db(engine):
    """Yield an AsyncSession tied to the per-test in-memory database."""
    factory = async_sessionmaker(engine, expire_on_commit=False)
    async with factory() as session:
        yield session


@pytest_asyncio.fixture(scope="function")
async def admin_user(db):
    """Create and persist a superadmin user, returning the ORM object."""
    now = datetime.now(timezone.utc)
    user = User(
        username="admin",
        email="admin@example.com",
        password_hash=hash_password("adminpass1"),
        role="superadmin",
        force_pw_change=False,
        is_active=True,
        created_at=now,
        updated_at=now,
    )
    db.add(user)
    await db.commit()
    await db.refresh(user)
    return user


@pytest_asyncio.fixture(scope="function")
async def viewer_user(db):
    """Create and persist a viewer user."""
    now = datetime.now(timezone.utc)
    user = User(
        username="viewer",
        email="viewer@example.com",
        password_hash=hash_password("viewerpass1"),
        role="viewer",
        force_pw_change=False,
        is_active=True,
        created_at=now,
        updated_at=now,
    )
    db.add(user)
    await db.commit()
    await db.refresh(user)
    return user


@pytest_asyncio.fixture(scope="function")
async def admin_token(admin_user):
    """Return a valid JWT access token for the admin user."""
    return create_access_token(admin_user.id, admin_user.role)


@pytest_asyncio.fixture(scope="function")
async def client(engine):
    """Return an AsyncClient wired to the FastAPI app with a test database.

    The scheduler is patched out so no real checks run, and the DB engine
    is overridden to use the per-test in-memory instance.
    """
    from app import main as app_main

    factory = async_sessionmaker(engine, expire_on_commit=False)

    async def override_get_db():
        async with factory() as session:
            yield session

    # Patch scheduler so it never starts during tests
    with (
        patch.object(app_main.scheduler, "start"),
        patch.object(app_main.scheduler, "shutdown"),
        patch("app.scheduler.manager.upsert_monitor_job"),
        patch("app.scheduler.manager.remove_monitor_job"),
        patch("app.scheduler.manager.pause_monitor_job"),
        patch("app.scheduler.manager.resume_monitor_job"),
        patch("app.scheduler.engine.scheduler"),
    ):
        from app.main import app
        from app.db.session import get_db
        app.dependency_overrides[get_db] = override_get_db

        # Seed admin user for the test client
        async with factory() as seed_db:
            now = datetime.now(timezone.utc)
            existing = await seed_db.execute(
                __import__("sqlalchemy", fromlist=["select"]).select(User).where(User.username == "admin")
            )
            if existing.scalar_one_or_none() is None:
                seed_db.add(User(
                    username="admin",
                    email="admin@example.com",
                    password_hash=hash_password("adminpass1"),
                    role="superadmin",
                    force_pw_change=False,
                    is_active=True,
                    created_at=now,
                    updated_at=now,
                ))
                await seed_db.commit()

        transport = ASGITransport(app=app)
        async with AsyncClient(transport=transport, base_url="http://test") as ac:
            yield ac

        app.dependency_overrides.clear()
