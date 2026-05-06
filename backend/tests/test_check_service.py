"""Integration tests for the incident state machine in check_service.py."""

import pytest
from datetime import datetime, timezone
from unittest.mock import patch, AsyncMock

from app.checkers.base import CheckResult
from app.db.models import Incident, Monitor
from app.services.check_service import process_result


def _make_monitor(db_session, user_id, **kwargs) -> Monitor:
    now = datetime.now(timezone.utc)
    m = Monitor(
        name=kwargs.get("name", "Test"),
        url=kwargs.get("url", "https://example.com"),
        monitor_type="http_get",
        check_interval_seconds=60,
        timeout_seconds=10,
        retry_count=kwargs.get("retry_count", 3),
        expected_status_codes="2xx",
        consecutive_failures=kwargs.get("consecutive_failures", 0),
        current_status="pending",
        alerts_enabled=kwargs.get("alerts_enabled", False),  # Off to avoid real notifications
        created_by=user_id,
        created_at=now,
        updated_at=now,
    )
    return m


class TestIncidentStateMachine:
    async def test_single_failure_no_incident(self, db, admin_user):
        monitor = _make_monitor(db, admin_user.id, retry_count=3)
        db.add(monitor)
        await db.flush()

        result = CheckResult(status="down", error_message="timeout")
        with patch("app.services.check_service.broadcaster") as mock_broadcaster:
            mock_broadcaster.publish = lambda *a, **kw: None
            await process_result(db, monitor, result)

        assert monitor.consecutive_failures == 1
        # No incident yet — need retry_count failures
        from sqlalchemy import select
        inc = (await db.execute(select(Incident).where(Incident.monitor_id == monitor.id))).scalar_one_or_none()
        assert inc is None

    async def test_failures_reach_retry_count_opens_incident(self, db, admin_user):
        monitor = _make_monitor(db, admin_user.id, retry_count=3, consecutive_failures=2)
        db.add(monitor)
        await db.flush()

        result = CheckResult(status="down", error_message="connection refused")
        with patch("app.services.check_service.broadcaster") as mock_broadcaster:
            mock_broadcaster.publish = lambda *a, **kw: None
            await process_result(db, monitor, result)

        assert monitor.consecutive_failures == 3

        from sqlalchemy import select
        inc = (await db.execute(select(Incident).where(Incident.monitor_id == monitor.id))).scalar_one_or_none()
        assert inc is not None
        assert inc.resolved_at is None

    async def test_recovery_closes_incident(self, db, admin_user):
        now = datetime.now(timezone.utc)
        monitor = _make_monitor(db, admin_user.id, retry_count=1, consecutive_failures=1)
        db.add(monitor)
        await db.flush()

        # Open an incident manually
        inc = Incident(monitor_id=monitor.id, started_at=now, root_cause="down")
        db.add(inc)
        await db.flush()

        result = CheckResult(status="up")
        with patch("app.services.check_service.broadcaster") as mock_broadcaster:
            mock_broadcaster.publish = lambda *a, **kw: None
            await process_result(db, monitor, result)

        await db.refresh(inc)
        assert inc.resolved_at is not None
        assert inc.duration_seconds is not None
        assert inc.duration_seconds >= 0

    async def test_consecutive_failures_reset_on_recovery(self, db, admin_user):
        monitor = _make_monitor(db, admin_user.id, consecutive_failures=5)
        db.add(monitor)
        await db.flush()

        result = CheckResult(status="up")
        with patch("app.services.check_service.broadcaster") as mock_broadcaster:
            mock_broadcaster.publish = lambda *a, **kw: None
            await process_result(db, monitor, result)

        assert monitor.consecutive_failures == 0

    async def test_status_updated_after_check(self, db, admin_user):
        monitor = _make_monitor(db, admin_user.id)
        db.add(monitor)
        await db.flush()

        result = CheckResult(status="warning", response_time_ms=3000)
        with patch("app.services.check_service.broadcaster") as mock_broadcaster:
            mock_broadcaster.publish = lambda *a, **kw: None
            await process_result(db, monitor, result)

        assert monitor.current_status == "warning"
        assert monitor.last_response_time_ms == 3000

    async def test_sse_broadcast_called(self, db, admin_user):
        monitor = _make_monitor(db, admin_user.id)
        db.add(monitor)
        await db.flush()

        result = CheckResult(status="up")
        published = []
        with patch("app.services.check_service.broadcaster") as mock_broadcaster:
            mock_broadcaster.publish = lambda event, data: published.append((event, data))
            await process_result(db, monitor, result)

        assert len(published) == 1
        event, data = published[0]
        assert event == "monitor.check_result"
        assert data["monitor_id"] == monitor.id

    async def test_down_notification_sent_with_alerts_enabled(self, db, admin_user):
        monitor = _make_monitor(db, admin_user.id, retry_count=1, alerts_enabled=True)
        db.add(monitor)
        await db.flush()

        result = CheckResult(status="down", error_message="timeout")
        with (
            patch("app.services.check_service.broadcaster") as mock_broadcaster,
            patch("app.services.check_service.notify_down", new_callable=AsyncMock) as mock_notify,
        ):
            mock_broadcaster.publish = lambda *a, **kw: None
            await process_result(db, monitor, result)
            mock_notify.assert_called_once()

    async def test_down_notification_exception_logged(self, db, admin_user):
        monitor = _make_monitor(db, admin_user.id, retry_count=1, alerts_enabled=True)
        db.add(monitor)
        await db.flush()

        result = CheckResult(status="down", error_message="timeout")
        with (
            patch("app.services.check_service.broadcaster") as mock_broadcaster,
            patch("app.services.check_service.notify_down", new_callable=AsyncMock, side_effect=Exception("SMTP err")),
        ):
            mock_broadcaster.publish = lambda *a, **kw: None
            await process_result(db, monitor, result)  # must not raise

    async def test_recovery_notification_sent_with_alerts_enabled(self, db, admin_user):
        now = datetime.now(timezone.utc)
        monitor = _make_monitor(db, admin_user.id, retry_count=1, consecutive_failures=1, alerts_enabled=True)
        db.add(monitor)
        await db.flush()

        inc = Incident(monitor_id=monitor.id, started_at=now, root_cause="down")
        db.add(inc)
        await db.flush()

        result = CheckResult(status="up")
        with (
            patch("app.services.check_service.broadcaster") as mock_broadcaster,
            patch("app.services.check_service.notify_recovery", new_callable=AsyncMock) as mock_notify,
        ):
            mock_broadcaster.publish = lambda *a, **kw: None
            await process_result(db, monitor, result)
            mock_notify.assert_called_once()

    async def test_recovery_notification_exception_logged(self, db, admin_user):
        now = datetime.now(timezone.utc)
        monitor = _make_monitor(db, admin_user.id, retry_count=1, consecutive_failures=1, alerts_enabled=True)
        db.add(monitor)
        await db.flush()

        inc = Incident(monitor_id=monitor.id, started_at=now, root_cause="down")
        db.add(inc)
        await db.flush()

        result = CheckResult(status="up")
        with (
            patch("app.services.check_service.broadcaster") as mock_broadcaster,
            patch("app.services.check_service.notify_recovery", new_callable=AsyncMock, side_effect=Exception("err")),
        ):
            mock_broadcaster.publish = lambda *a, **kw: None
            await process_result(db, monitor, result)  # must not raise
