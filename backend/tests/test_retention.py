"""Tests for the data retention service."""

from datetime import datetime, timedelta, timezone
from unittest.mock import patch

from app.db.models import AppSetting, CheckResult, Monitor
from app.services.retention_service import run_retention_cleanup


async def _seed_data(db, admin_user, n_old: int, n_recent: int, cutoff_days: int = 5):
    """Create monitor and check results, some old and some recent."""
    now = datetime.now(timezone.utc)
    m = Monitor(
        name="Ret Monitor",
        url="https://ret.example.com",
        monitor_type="http_get",
        check_interval_seconds=60,
        timeout_seconds=10,
        retry_count=3,
        expected_status_codes="2xx",
        current_status="up",
        created_by=admin_user.id,
        created_at=now,
        updated_at=now,
    )
    db.add(m)
    await db.flush()

    for i in range(n_old):
        db.add(CheckResult(
            monitor_id=m.id,
            checked_at=now - timedelta(days=cutoff_days + 1 + i),
            status="up",
            response_time_ms=100,
        ))
    for i in range(n_recent):
        db.add(CheckResult(
            monitor_id=m.id,
            checked_at=now - timedelta(hours=i),
            status="up",
            response_time_ms=100,
        ))
    await db.commit()
    return m


class TestRetentionService:
    async def test_deletes_old_rows(self, db, admin_user):
        await _seed_data(db, admin_user, n_old=5, n_recent=3, cutoff_days=5)

        # Set retention to 5 days via AppSetting
        now = datetime.now(timezone.utc)
        db.add(AppSetting(key="data_retention_days", value="5", updated_at=now))
        await db.commit()

        # run_retention_cleanup uses its own session — patch AsyncSessionLocal
        from sqlalchemy.ext.asyncio import async_sessionmaker, create_async_engine
        from app.db.session import Base

        engine = create_async_engine("sqlite+aiosqlite:///:memory:", echo=False)
        async with engine.begin() as conn:
            await conn.run_sync(Base.metadata.create_all)
        factory = async_sessionmaker(engine, expire_on_commit=False)

        # Re-seed in the new engine
        async with factory() as s:
            now = datetime.now(timezone.utc)
            m = Monitor(name="R", url="https://r.example.com", monitor_type="http_get",
                        check_interval_seconds=60, timeout_seconds=10, retry_count=3,
                        expected_status_codes="2xx", current_status="up",
                        created_by=1, created_at=now, updated_at=now)
            s.add(m)
            await s.flush()
            s.add(AppSetting(key="data_retention_days", value="5", updated_at=now))
            # Add old rows
            for i in range(3):
                s.add(CheckResult(monitor_id=m.id, checked_at=now - timedelta(days=10 + i),
                                   status="up"))
            # Add recent rows
            s.add(CheckResult(monitor_id=m.id, checked_at=now - timedelta(hours=1), status="up"))
            await s.commit()

        with patch("app.services.retention_service.AsyncSessionLocal", factory):
            await run_retention_cleanup()

        # Verify old rows deleted
        from sqlalchemy import select, func
        async with factory() as s:
            count = (await s.execute(select(func.count()).select_from(CheckResult))).scalar_one()
        assert count == 1  # Only the recent row remains

        await engine.dispose()

    async def test_no_setting_uses_default_90_days(self, db, admin_user):
        """When no data_retention_days setting exists, should default to 90 days."""
        from sqlalchemy.ext.asyncio import async_sessionmaker, create_async_engine
        from app.db.session import Base
        from sqlalchemy import select, func

        engine = create_async_engine("sqlite+aiosqlite:///:memory:", echo=False)
        async with engine.begin() as conn:
            await conn.run_sync(Base.metadata.create_all)
        factory = async_sessionmaker(engine, expire_on_commit=False)

        async with factory() as s:
            now = datetime.now(timezone.utc)
            m = Monitor(name="R2", url="https://r2.example.com", monitor_type="http_get",
                        check_interval_seconds=60, timeout_seconds=10, retry_count=3,
                        expected_status_codes="2xx", current_status="up",
                        created_by=1, created_at=now, updated_at=now)
            s.add(m)
            await s.flush()
            # Add one row that's 100 days old (should be deleted with 90-day default)
            s.add(CheckResult(monitor_id=m.id, checked_at=now - timedelta(days=100), status="up"))
            # Add one row that's 10 days old (should be kept)
            s.add(CheckResult(monitor_id=m.id, checked_at=now - timedelta(days=10), status="up"))
            await s.commit()

        with patch("app.services.retention_service.AsyncSessionLocal", factory):
            await run_retention_cleanup()

        async with factory() as s:
            count = (await s.execute(select(func.count()).select_from(CheckResult))).scalar_one()
        assert count == 1

        await engine.dispose()
