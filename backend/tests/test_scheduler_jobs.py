"""Tests for APScheduler job functions.

These tests call the job functions directly (not via the scheduler) and
patch ``AsyncSessionLocal`` with an in-memory session factory so that real
DB I/O occurs without touching the production database.
"""

import pytest
from datetime import datetime, timedelta, timezone
from unittest.mock import AsyncMock, MagicMock, patch

from sqlalchemy.ext.asyncio import async_sessionmaker, create_async_engine

from app.checkers.base import CheckResult
from app.db.models import MaintenanceWindow, MaintenanceWindowMonitor, Monitor
from app.db.session import Base


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

async def _make_engine():
    engine = create_async_engine("sqlite+aiosqlite:///:memory:", echo=False)
    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)
    return engine


async def _make_monitor(factory, **kwargs):
    """Insert a Monitor row and return its id."""
    now = datetime.now(timezone.utc)
    defaults = dict(
        name="Test Monitor",
        url="https://example.com",
        monitor_type="http_get",
        check_interval_seconds=60,
        timeout_seconds=10,
        retry_count=3,
        expected_status_codes="2xx",
        is_enabled=True,
        current_status="up",
        consecutive_failures=0,
        ssl_check_enabled=False,
        ssl_expiry_warning_days=30,
        alerts_enabled=True,
        alert_cooldown_seconds=1800,
        created_at=now,
        updated_at=now,
    )
    defaults.update(kwargs)
    async with factory() as s:
        m = Monitor(**defaults)
        s.add(m)
        await s.commit()
        await s.refresh(m)
        return m.id


# ---------------------------------------------------------------------------
# run_monitor_check
# ---------------------------------------------------------------------------

class TestRunMonitorCheck:
    async def test_skips_disabled_monitor(self):
        engine = await _make_engine()
        factory = async_sessionmaker(engine, expire_on_commit=False)
        mid = await _make_monitor(factory, is_enabled=False)

        with (
            patch("app.scheduler.jobs.AsyncSessionLocal", factory),
            patch("app.scheduler.jobs.http_check", new_callable=AsyncMock) as mock_http,
            patch("app.scheduler.jobs.process_result", new_callable=AsyncMock),
        ):
            from app.scheduler.jobs import run_monitor_check
            await run_monitor_check(mid)
            mock_http.assert_not_called()

        await engine.dispose()

    async def test_skips_nonexistent_monitor(self):
        engine = await _make_engine()
        factory = async_sessionmaker(engine, expire_on_commit=False)

        with (
            patch("app.scheduler.jobs.AsyncSessionLocal", factory),
            patch("app.scheduler.jobs.http_check", new_callable=AsyncMock) as mock_http,
        ):
            from app.scheduler.jobs import run_monitor_check
            await run_monitor_check(99999)
            mock_http.assert_not_called()

        await engine.dispose()

    async def test_http_monitor_runs_http_check(self):
        engine = await _make_engine()
        factory = async_sessionmaker(engine, expire_on_commit=False)
        mid = await _make_monitor(factory, monitor_type="http_get", url="https://example.com")

        mock_result = CheckResult(status="up", response_time_ms=100)

        with (
            patch("app.scheduler.jobs.AsyncSessionLocal", factory),
            patch("app.scheduler.jobs.http_check", new_callable=AsyncMock, return_value=mock_result) as mock_http,
            patch("app.scheduler.jobs.process_result", new_callable=AsyncMock),
        ):
            from app.scheduler.jobs import run_monitor_check
            await run_monitor_check(mid)
            mock_http.assert_called_once()

        await engine.dispose()

    async def test_ping_monitor_runs_ping_check(self):
        engine = await _make_engine()
        factory = async_sessionmaker(engine, expire_on_commit=False)
        mid = await _make_monitor(factory, monitor_type="ping", url="ping://example.com")

        mock_result = CheckResult(status="up", response_time_ms=10)

        with (
            patch("app.scheduler.jobs.AsyncSessionLocal", factory),
            patch("app.scheduler.jobs.ping_check", new_callable=AsyncMock, return_value=mock_result) as mock_ping,
            patch("app.scheduler.jobs.process_result", new_callable=AsyncMock),
        ):
            from app.scheduler.jobs import run_monitor_check
            await run_monitor_check(mid)
            mock_ping.assert_called_once()

        await engine.dispose()

    async def test_heartbeat_monitor_skips_outbound_check(self):
        engine = await _make_engine()
        factory = async_sessionmaker(engine, expire_on_commit=False)
        mid = await _make_monitor(
            factory,
            monitor_type="heartbeat",
            url="heartbeat://app",
            heartbeat_token="tok-abc",
            heartbeat_grace_seconds=60,
            heartbeat_last_ping=None,
        )

        with (
            patch("app.scheduler.jobs.AsyncSessionLocal", factory),
            patch("app.scheduler.jobs.http_check", new_callable=AsyncMock) as mock_http,
            patch("app.scheduler.jobs.process_result", new_callable=AsyncMock),
        ):
            from app.scheduler.jobs import run_monitor_check
            await run_monitor_check(mid)
            mock_http.assert_not_called()

        await engine.dispose()

    async def test_ssl_check_triggered_for_https(self):
        engine = await _make_engine()
        factory = async_sessionmaker(engine, expire_on_commit=False)
        mid = await _make_monitor(
            factory, monitor_type="http_get",
            url="https://example.com", ssl_check_enabled=True,
        )

        mock_result = CheckResult(status="up", response_time_ms=100)

        with (
            patch("app.scheduler.jobs.AsyncSessionLocal", factory),
            patch("app.scheduler.jobs.http_check", new_callable=AsyncMock, return_value=mock_result),
            patch("app.scheduler.jobs.ssl_check", new_callable=AsyncMock, return_value=(True, 60, None)) as mock_ssl,
            patch("app.scheduler.jobs.process_result", new_callable=AsyncMock),
        ):
            from app.scheduler.jobs import run_monitor_check
            await run_monitor_check(mid)
            mock_ssl.assert_called_once()

        await engine.dispose()

    async def test_ssl_warning_sends_notification(self):
        engine = await _make_engine()
        factory = async_sessionmaker(engine, expire_on_commit=False)
        mid = await _make_monitor(
            factory, monitor_type="http_get",
            url="https://example.com", ssl_check_enabled=True,
            ssl_expiry_warning_days=30,
        )

        mock_result = CheckResult(status="up", response_time_ms=100)

        with (
            patch("app.scheduler.jobs.AsyncSessionLocal", factory),
            patch("app.scheduler.jobs.http_check", new_callable=AsyncMock, return_value=mock_result),
            patch("app.scheduler.jobs.ssl_check", new_callable=AsyncMock, return_value=(True, 10, None)),
            patch("app.scheduler.jobs.notify_ssl_warning", new_callable=AsyncMock) as mock_notify,
            patch("app.scheduler.jobs.process_result", new_callable=AsyncMock),
        ):
            from app.scheduler.jobs import run_monitor_check
            await run_monitor_check(mid)
            mock_notify.assert_called_once()

        await engine.dispose()

    async def test_ssl_warning_exception_logged(self):
        engine = await _make_engine()
        factory = async_sessionmaker(engine, expire_on_commit=False)
        mid = await _make_monitor(
            factory, monitor_type="http_get",
            url="https://example.com", ssl_check_enabled=True,
            ssl_expiry_warning_days=30,
        )

        mock_result = CheckResult(status="up", response_time_ms=100)

        with (
            patch("app.scheduler.jobs.AsyncSessionLocal", factory),
            patch("app.scheduler.jobs.http_check", new_callable=AsyncMock, return_value=mock_result),
            patch("app.scheduler.jobs.ssl_check", new_callable=AsyncMock, return_value=(True, 10, None)),
            patch("app.scheduler.jobs.notify_ssl_warning", new_callable=AsyncMock, side_effect=Exception("notify error")),
            patch("app.scheduler.jobs.process_result", new_callable=AsyncMock),
        ):
            from app.scheduler.jobs import run_monitor_check
            await run_monitor_check(mid)  # must not raise

        await engine.dispose()

    async def test_in_maintenance_skips_check(self):
        engine = await _make_engine()
        factory = async_sessionmaker(engine, expire_on_commit=False)
        mid = await _make_monitor(factory)

        now = datetime.now(timezone.utc)
        async with factory() as s:
            w = MaintenanceWindow(
                name="Maint",
                starts_at=now - timedelta(hours=1),
                ends_at=now + timedelta(hours=1),
                suppress_alerts=True,
                created_by=1,
                created_at=now,
            )
            s.add(w)
            await s.flush()
            s.add(MaintenanceWindowMonitor(window_id=w.id, monitor_id=mid))
            await s.commit()

        with (
            patch("app.scheduler.jobs.AsyncSessionLocal", factory),
            patch("app.scheduler.jobs.http_check", new_callable=AsyncMock) as mock_http,
            patch("app.scheduler.jobs.process_result", new_callable=AsyncMock),
        ):
            from app.scheduler.jobs import run_monitor_check
            await run_monitor_check(mid)
            mock_http.assert_not_called()

        await engine.dispose()


# ---------------------------------------------------------------------------
# _perform_check
# ---------------------------------------------------------------------------

class TestPerformCheck:
    async def test_ping_type(self):
        from app.scheduler.jobs import _perform_check

        monitor = MagicMock()
        monitor.monitor_type = "ping"
        monitor.url = "ping://host.example.com"
        monitor.timeout_seconds = 5
        monitor.response_time_warning_ms = 200

        mock_result = CheckResult(status="up")
        with patch("app.scheduler.jobs.ping_check", new_callable=AsyncMock, return_value=mock_result):
            result = await _perform_check(monitor)
        assert result.status == "up"

    async def test_http_post_type(self):
        from app.scheduler.jobs import _perform_check

        monitor = MagicMock()
        monitor.monitor_type = "http_post"
        monitor.url = "https://example.com/api"
        monitor.timeout_seconds = 10
        monitor.response_time_warning_ms = 2000
        monitor.request_headers = None
        monitor.request_body = None
        monitor.expected_status_codes = "2xx"
        monitor.expected_body_type = None
        monitor.expected_body_value = None

        mock_result = CheckResult(status="up")
        with patch("app.scheduler.jobs.http_check", new_callable=AsyncMock, return_value=mock_result) as mock_http:
            result = await _perform_check(monitor)
            assert mock_http.call_args.kwargs["method"] == "POST"
        assert result.status == "up"

    async def test_http_head_type(self):
        from app.scheduler.jobs import _perform_check

        monitor = MagicMock()
        monitor.monitor_type = "http_head"
        monitor.url = "https://example.com"
        monitor.timeout_seconds = 10
        monitor.response_time_warning_ms = 2000
        monitor.request_headers = None
        monitor.request_body = None
        monitor.expected_status_codes = "2xx"
        monitor.expected_body_type = None
        monitor.expected_body_value = None

        mock_result = CheckResult(status="up")
        with patch("app.scheduler.jobs.http_check", new_callable=AsyncMock, return_value=mock_result) as mock_http:
            await _perform_check(monitor)
            assert mock_http.call_args.kwargs["method"] == "HEAD"

    async def test_unknown_type_defaults_to_get(self):
        from app.scheduler.jobs import _perform_check

        monitor = MagicMock()
        monitor.monitor_type = "unknown_type"
        monitor.url = "https://example.com"
        monitor.timeout_seconds = 10
        monitor.response_time_warning_ms = 2000
        monitor.request_headers = None
        monitor.request_body = None
        monitor.expected_status_codes = "2xx"
        monitor.expected_body_type = None
        monitor.expected_body_value = None

        mock_result = CheckResult(status="up")
        with patch("app.scheduler.jobs.http_check", new_callable=AsyncMock, return_value=mock_result) as mock_http:
            await _perform_check(monitor)
            assert mock_http.call_args.kwargs["method"] == "GET"


# ---------------------------------------------------------------------------
# _check_heartbeat
# ---------------------------------------------------------------------------

class TestCheckHeartbeat:
    async def test_down_when_no_last_ping(self):
        from app.scheduler.jobs import _check_heartbeat

        monitor = MagicMock()
        monitor.heartbeat_last_ping = None
        monitor.heartbeat_grace_seconds = 60
        monitor.check_interval_seconds = 300

        mock_db = MagicMock()
        with patch("app.scheduler.jobs.process_result", new_callable=AsyncMock) as mock_pr:
            await _check_heartbeat(mock_db, monitor)
            result = mock_pr.call_args.args[2]
            assert result.status == "down"
            assert "not received" in result.error_message.lower()

    async def test_down_when_stale_ping(self):
        from app.scheduler.jobs import _check_heartbeat

        now = datetime.now(timezone.utc)
        monitor = MagicMock()
        monitor.heartbeat_last_ping = now - timedelta(hours=10)
        monitor.heartbeat_grace_seconds = 60
        monitor.check_interval_seconds = 300

        mock_db = MagicMock()
        with patch("app.scheduler.jobs.process_result", new_callable=AsyncMock) as mock_pr:
            await _check_heartbeat(mock_db, monitor)
            result = mock_pr.call_args.args[2]
            assert result.status == "down"

    async def test_up_when_recent_ping(self):
        from app.scheduler.jobs import _check_heartbeat

        now = datetime.now(timezone.utc)
        monitor = MagicMock()
        monitor.heartbeat_last_ping = now - timedelta(seconds=30)
        monitor.heartbeat_grace_seconds = 60
        monitor.check_interval_seconds = 300

        mock_db = MagicMock()
        with patch("app.scheduler.jobs.process_result", new_callable=AsyncMock) as mock_pr:
            await _check_heartbeat(mock_db, monitor)
            result = mock_pr.call_args.args[2]
            assert result.status == "up"


# ---------------------------------------------------------------------------
# _in_maintenance
# ---------------------------------------------------------------------------

class TestInMaintenance:
    async def test_not_in_maintenance_when_no_windows(self):
        engine = await _make_engine()
        factory = async_sessionmaker(engine, expire_on_commit=False)

        from app.scheduler.jobs import _in_maintenance
        async with factory() as db:
            result = await _in_maintenance(db, 999)
        assert result is False

        await engine.dispose()

    async def test_in_active_maintenance_window(self):
        engine = await _make_engine()
        factory = async_sessionmaker(engine, expire_on_commit=False)
        mid = await _make_monitor(factory)

        now = datetime.now(timezone.utc)
        async with factory() as s:
            w = MaintenanceWindow(
                name="MW",
                starts_at=now - timedelta(hours=1),
                ends_at=now + timedelta(hours=1),
                suppress_alerts=True,
                created_by=1,
                created_at=now,
            )
            s.add(w)
            await s.flush()
            s.add(MaintenanceWindowMonitor(window_id=w.id, monitor_id=mid))
            await s.commit()

        from app.scheduler.jobs import _in_maintenance
        async with factory() as db:
            result = await _in_maintenance(db, mid)
        assert result is True

        await engine.dispose()

    async def test_not_in_past_maintenance_window(self):
        engine = await _make_engine()
        factory = async_sessionmaker(engine, expire_on_commit=False)
        mid = await _make_monitor(factory)

        now = datetime.now(timezone.utc)
        async with factory() as s:
            w = MaintenanceWindow(
                name="Old",
                starts_at=now - timedelta(hours=5),
                ends_at=now - timedelta(hours=1),  # already ended
                suppress_alerts=True,
                created_by=1,
                created_at=now - timedelta(hours=6),
            )
            s.add(w)
            await s.flush()
            s.add(MaintenanceWindowMonitor(window_id=w.id, monitor_id=mid))
            await s.commit()

        from app.scheduler.jobs import _in_maintenance
        async with factory() as db:
            result = await _in_maintenance(db, mid)
        assert result is False

        await engine.dispose()


# ---------------------------------------------------------------------------
# run_daily_retention / run_daily_ssl_scan
# ---------------------------------------------------------------------------

class TestDailyJobs:
    async def test_run_daily_retention_calls_cleanup(self):
        with patch("app.scheduler.jobs.run_retention_cleanup", new_callable=AsyncMock) as mock_cleanup:
            from app.scheduler.jobs import run_daily_retention
            await run_daily_retention()
            mock_cleanup.assert_called_once()

    async def test_run_daily_retention_handles_exception(self):
        with patch(
            "app.scheduler.jobs.run_retention_cleanup",
            new_callable=AsyncMock,
            side_effect=Exception("DB error"),
        ):
            from app.scheduler.jobs import run_daily_retention
            await run_daily_retention()  # must not raise

    async def test_run_daily_ssl_scan_notifies_expiring(self):
        engine = await _make_engine()
        factory = async_sessionmaker(engine, expire_on_commit=False)
        await _make_monitor(
            factory,
            monitor_type="http_get",
            url="https://example.com",
            ssl_check_enabled=True,
            ssl_expiry_warning_days=30,
            is_enabled=True,
        )

        with (
            patch("app.scheduler.jobs.AsyncSessionLocal", factory),
            patch("app.scheduler.jobs.ssl_check", new_callable=AsyncMock, return_value=(True, 10, None)),
            patch("app.scheduler.jobs.notify_ssl_warning", new_callable=AsyncMock) as mock_notify,
        ):
            from app.scheduler.jobs import run_daily_ssl_scan
            await run_daily_ssl_scan()
            mock_notify.assert_called_once()

        await engine.dispose()

    async def test_run_daily_ssl_scan_skips_http_url(self):
        engine = await _make_engine()
        factory = async_sessionmaker(engine, expire_on_commit=False)
        await _make_monitor(
            factory,
            monitor_type="http_get",
            url="http://example.com",  # not https
            ssl_check_enabled=True,
            ssl_expiry_warning_days=30,
            is_enabled=True,
        )

        with (
            patch("app.scheduler.jobs.AsyncSessionLocal", factory),
            patch("app.scheduler.jobs.ssl_check", new_callable=AsyncMock) as mock_ssl,
        ):
            from app.scheduler.jobs import run_daily_ssl_scan
            await run_daily_ssl_scan()
            mock_ssl.assert_not_called()

        await engine.dispose()

    async def test_run_daily_ssl_scan_handles_exception(self):
        engine = await _make_engine()
        factory = async_sessionmaker(engine, expire_on_commit=False)
        await _make_monitor(
            factory,
            monitor_type="http_get",
            url="https://example.com",
            ssl_check_enabled=True,
            ssl_expiry_warning_days=30,
            is_enabled=True,
        )

        with (
            patch("app.scheduler.jobs.AsyncSessionLocal", factory),
            patch("app.scheduler.jobs.ssl_check", new_callable=AsyncMock, side_effect=Exception("SSL error")),
        ):
            from app.scheduler.jobs import run_daily_ssl_scan
            await run_daily_ssl_scan()  # must not raise

        await engine.dispose()

    async def test_run_daily_ssl_scan_no_warn_when_days_ok(self):
        engine = await _make_engine()
        factory = async_sessionmaker(engine, expire_on_commit=False)
        await _make_monitor(
            factory,
            monitor_type="http_get",
            url="https://example.com",
            ssl_check_enabled=True,
            ssl_expiry_warning_days=30,
            is_enabled=True,
        )

        with (
            patch("app.scheduler.jobs.AsyncSessionLocal", factory),
            # 90 days left → no warning needed (threshold is 30)
            patch("app.scheduler.jobs.ssl_check", new_callable=AsyncMock, return_value=(True, 90, None)),
            patch("app.scheduler.jobs.notify_ssl_warning", new_callable=AsyncMock) as mock_notify,
        ):
            from app.scheduler.jobs import run_daily_ssl_scan
            await run_daily_ssl_scan()
            mock_notify.assert_not_called()

        await engine.dispose()
