"""Integration tests for /api/incidents/* endpoints."""

from datetime import datetime, timezone

from app.db.models import Incident, Monitor


async def _login(client):
    resp = await client.post("/api/auth/login", json={"username": "admin", "password": "adminpass1"})
    return {"Authorization": f"Bearer {resp.json()['access_token']}"}


async def _seed_monitor_and_incident(db, admin_user):
    """Helper: create a monitor and an open incident in the test DB."""
    now = datetime.now(timezone.utc)
    monitor = Monitor(
        name="Seed Monitor",
        url="https://seed.example.com",
        monitor_type="http_get",
        check_interval_seconds=60,
        timeout_seconds=10,
        retry_count=3,
        expected_status_codes="2xx",
        current_status="down",
        consecutive_failures=3,
        created_by=admin_user.id,
        created_at=now,
        updated_at=now,
    )
    db.add(monitor)
    await db.flush()

    incident = Incident(
        monitor_id=monitor.id,
        started_at=now,
        root_cause="Connection refused",
    )
    db.add(incident)
    await db.commit()
    await db.refresh(incident)
    return monitor, incident


class TestListIncidents:
    async def test_empty_list(self, client):
        headers = await _login(client)
        resp = await client.get("/api/incidents", headers=headers)
        assert resp.status_code == 200
        assert resp.json() == []

    async def test_unauthenticated(self, client):
        resp = await client.get("/api/incidents")
        assert resp.status_code == 401


class TestGetIncident:
    async def test_not_found(self, client):
        headers = await _login(client)
        resp = await client.get("/api/incidents/99999", headers=headers)
        assert resp.status_code == 404


class TestAcknowledgeIncident:
    async def test_acknowledge_not_found(self, client):
        headers = await _login(client)
        resp = await client.post("/api/incidents/99999/acknowledge", headers=headers)
        assert resp.status_code == 404
