"""Tests for notification_service: down, recovery, SSL warning dispatch."""

import json
import pytest
from datetime import datetime, timezone
from unittest.mock import AsyncMock, patch

from app.db.models import (
    Incident, Monitor, MonitorNotificationChannel, NotificationChannel
)
from app.services.notification_service import (
    _fmt_duration,
    notify_down,
    notify_recovery,
    notify_ssl_warning,
)


class TestFmtDuration:
    def test_seconds(self):
        assert _fmt_duration(45) == "45s"

    def test_minutes(self):
        assert _fmt_duration(90) == "1m 30s"

    def test_hours(self):
        assert _fmt_duration(3661) == "1h 1m"

    def test_days(self):
        assert _fmt_duration(90061) == "1d 1h 1m"

    def test_none(self):
        assert _fmt_duration(None) == "unknown duration"

    def test_zero(self):
        assert _fmt_duration(0) == "unknown duration"


class TestNotifyDispatch:
    def _make_monitor(self, db, user_id):
        now = datetime.now(timezone.utc)
        m = Monitor(
            name="Test",
            url="https://example.com",
            monitor_type="http_get",
            check_interval_seconds=60,
            timeout_seconds=10,
            retry_count=3,
            expected_status_codes="2xx",
            current_status="down",
            alerts_enabled=True,
            created_by=user_id,
            created_at=now,
            updated_at=now,
        )
        return m

    async def test_notify_down_sends_email(self, db, admin_user):
        now = datetime.now(timezone.utc)
        monitor = self._make_monitor(db, admin_user.id)
        db.add(monitor)
        await db.flush()

        cfg = json.dumps({
            "smtp_host": "smtp.example.com",
            "smtp_port": 587,
            "smtp_user": "user",
            "smtp_password_enc": "",
            "smtp_from": "noreply@example.com",
            "recipients": ["admin@example.com"],
            "use_tls": False,
        })
        channel = NotificationChannel(
            name="email-ch",
            channel_type="email",
            config=cfg,
            is_enabled=True,
            created_by=admin_user.id,
            created_at=now,
            updated_at=now,
        )
        db.add(channel)
        await db.flush()

        db.add(MonitorNotificationChannel(
            monitor_id=monitor.id,
            channel_id=channel.id,
            on_down=True,
            on_recovery=True,
            on_ssl_warn=True,
        ))
        await db.flush()

        incident = Incident(
            monitor_id=monitor.id,
            started_at=now,
            root_cause="timeout",
        )
        db.add(incident)
        await db.flush()

        with patch("app.services.notification_service.send_email", new_callable=AsyncMock) as mock_email:
            mock_email.return_value = True
            await notify_down(db, monitor, incident)
            mock_email.assert_called_once()

    async def test_notify_recovery_sends_email(self, db, admin_user):
        now = datetime.now(timezone.utc)
        monitor = self._make_monitor(db, admin_user.id)
        db.add(monitor)
        await db.flush()

        cfg = json.dumps({"smtp_host": "smtp.example.com", "smtp_port": 587,
                           "recipients": ["a@example.com"], "use_tls": False,
                           "smtp_user": "", "smtp_password_enc": "", "smtp_from": ""})
        channel = NotificationChannel(name="c", channel_type="email", config=cfg,
                                      is_enabled=True, created_by=admin_user.id,
                                      created_at=now, updated_at=now)
        db.add(channel)
        await db.flush()

        db.add(MonitorNotificationChannel(monitor_id=monitor.id, channel_id=channel.id,
                                          on_down=True, on_recovery=True, on_ssl_warn=True))
        await db.flush()

        incident = Incident(
            monitor_id=monitor.id,
            started_at=now,
            resolved_at=now,
            duration_seconds=120,
            root_cause="timeout",
        )
        db.add(incident)
        await db.flush()

        with patch("app.services.notification_service.send_email", new_callable=AsyncMock) as mock_email:
            mock_email.return_value = True
            await notify_recovery(db, monitor, incident)
            mock_email.assert_called_once()

    async def test_notify_ssl_warning_sends_telegram(self, db, admin_user):
        now = datetime.now(timezone.utc)
        monitor = self._make_monitor(db, admin_user.id)
        db.add(monitor)
        await db.flush()

        cfg = json.dumps({"bot_token": "token123", "chat_id": "99999"})
        channel = NotificationChannel(name="tg", channel_type="telegram", config=cfg,
                                      is_enabled=True, created_by=admin_user.id,
                                      created_at=now, updated_at=now)
        db.add(channel)
        await db.flush()

        db.add(MonitorNotificationChannel(monitor_id=monitor.id, channel_id=channel.id,
                                          on_down=True, on_recovery=True, on_ssl_warn=True))
        await db.flush()

        with patch("app.services.notification_service.send_telegram", new_callable=AsyncMock) as mock_tg:
            mock_tg.return_value = True
            await notify_ssl_warning(db, monitor, 7)
            mock_tg.assert_called_once()

    async def test_disabled_channel_not_notified(self, db, admin_user):
        now = datetime.now(timezone.utc)
        monitor = self._make_monitor(db, admin_user.id)
        db.add(monitor)
        await db.flush()

        cfg = json.dumps({"smtp_host": "x", "recipients": ["x@example.com"]})
        channel = NotificationChannel(name="c", channel_type="email", config=cfg,
                                      is_enabled=False,  # disabled
                                      created_by=admin_user.id, created_at=now, updated_at=now)
        db.add(channel)
        await db.flush()

        db.add(MonitorNotificationChannel(monitor_id=monitor.id, channel_id=channel.id,
                                          on_down=True, on_recovery=True, on_ssl_warn=True))
        await db.flush()

        incident = Incident(monitor_id=monitor.id, started_at=now, root_cause="x")
        db.add(incident)
        await db.flush()

        with patch("app.services.notification_service.send_email", new_callable=AsyncMock) as mock_email:
            await notify_down(db, monitor, incident)
            mock_email.assert_not_called()

    async def test_on_down_false_skipped(self, db, admin_user):
        now = datetime.now(timezone.utc)
        monitor = self._make_monitor(db, admin_user.id)
        db.add(monitor)
        await db.flush()

        cfg = json.dumps({"smtp_host": "x", "recipients": ["x@example.com"]})
        channel = NotificationChannel(name="c", channel_type="email", config=cfg,
                                      is_enabled=True, created_by=admin_user.id,
                                      created_at=now, updated_at=now)
        db.add(channel)
        await db.flush()

        db.add(MonitorNotificationChannel(monitor_id=monitor.id, channel_id=channel.id,
                                          on_down=False,  # disabled for down events
                                          on_recovery=True, on_ssl_warn=True))
        await db.flush()

        incident = Incident(monitor_id=monitor.id, started_at=now, root_cause="x")
        db.add(incident)
        await db.flush()

        with patch("app.services.notification_service.send_email", new_callable=AsyncMock) as mock_email:
            await notify_down(db, monitor, incident)
            mock_email.assert_not_called()
