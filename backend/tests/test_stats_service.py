"""Tests for stats_service: uptime %, avg RT, incident counts."""

from datetime import datetime, timedelta, timezone

from app.db.models import CheckResult, Incident, Monitor
from app.services.stats_service import get_monitor_stats


async def _seed_monitor(db, user_id) -> Monitor:
    now = datetime.now(timezone.utc)
    m = Monitor(
        name="Stats Monitor",
        url="https://stats.example.com",
        monitor_type="http_get",
        check_interval_seconds=60,
        timeout_seconds=10,
        retry_count=3,
        expected_status_codes="2xx",
        current_status="up",
        created_by=user_id,
        created_at=now,
        updated_at=now,
    )
    db.add(m)
    await db.flush()
    return m


class TestGetMonitorStats:
    async def test_no_checks_returns_100_uptime(self, db, admin_user):
        monitor = await _seed_monitor(db, admin_user.id)
        stats = await get_monitor_stats(db, monitor.id, days=30)
        assert stats.uptime_percent == 100.0
        assert stats.total_checks == 0
        assert stats.total_incidents == 0

    async def test_all_up_checks(self, db, admin_user):
        monitor = await _seed_monitor(db, admin_user.id)
        now = datetime.now(timezone.utc)
        for i in range(10):
            db.add(CheckResult(
                monitor_id=monitor.id,
                checked_at=now - timedelta(hours=i),
                status="up",
                response_time_ms=100 + i * 10,
            ))
        await db.commit()

        stats = await get_monitor_stats(db, monitor.id, days=30)
        assert stats.uptime_percent == 100.0
        assert stats.total_checks == 10
        assert stats.avg_response_time_ms is not None

    async def test_mixed_up_down(self, db, admin_user):
        monitor = await _seed_monitor(db, admin_user.id)
        now = datetime.now(timezone.utc)
        for i in range(8):
            db.add(CheckResult(
                monitor_id=monitor.id,
                checked_at=now - timedelta(hours=i),
                status="up",
                response_time_ms=100,
            ))
        for i in range(2):
            db.add(CheckResult(
                monitor_id=monitor.id,
                checked_at=now - timedelta(hours=10 + i),
                status="down",
            ))
        await db.commit()

        stats = await get_monitor_stats(db, monitor.id, days=30)
        assert stats.total_checks == 10
        assert stats.uptime_percent == 80.0

    async def test_warning_counts_as_up(self, db, admin_user):
        monitor = await _seed_monitor(db, admin_user.id)
        now = datetime.now(timezone.utc)
        db.add(CheckResult(monitor_id=monitor.id, checked_at=now, status="up", response_time_ms=200))
        db.add(CheckResult(monitor_id=monitor.id, checked_at=now - timedelta(hours=1), status="warning", response_time_ms=3000))
        await db.commit()

        stats = await get_monitor_stats(db, monitor.id, days=30)
        assert stats.uptime_percent == 100.0

    async def test_incident_count(self, db, admin_user):
        monitor = await _seed_monitor(db, admin_user.id)
        now = datetime.now(timezone.utc)
        for _ in range(3):
            db.add(Incident(
                monitor_id=monitor.id,
                started_at=now,
                resolved_at=now + timedelta(minutes=5),
                duration_seconds=300,
            ))
        await db.commit()

        stats = await get_monitor_stats(db, monitor.id, days=30)
        assert stats.total_incidents == 3

    async def test_open_incident_count(self, db, admin_user):
        monitor = await _seed_monitor(db, admin_user.id)
        now = datetime.now(timezone.utc)
        db.add(Incident(monitor_id=monitor.id, started_at=now))  # open
        db.add(Incident(monitor_id=monitor.id, started_at=now - timedelta(hours=2),
                        resolved_at=now, duration_seconds=7200))  # closed
        await db.commit()

        stats = await get_monitor_stats(db, monitor.id, days=30)
        assert stats.total_incidents == 2
        assert stats.open_incidents == 1

    async def test_period_days_respected(self, db, admin_user):
        monitor = await _seed_monitor(db, admin_user.id)
        now = datetime.now(timezone.utc)
        # Check inside window
        db.add(CheckResult(monitor_id=monitor.id, checked_at=now - timedelta(days=5), status="up"))
        # Check outside window
        db.add(CheckResult(monitor_id=monitor.id, checked_at=now - timedelta(days=40), status="down"))
        await db.commit()

        stats = await get_monitor_stats(db, monitor.id, days=30)
        assert stats.total_checks == 1
        assert stats.uptime_percent == 100.0
