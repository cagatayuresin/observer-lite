"""Integration tests for maintenance window CRUD endpoints."""

import pytest
from datetime import datetime, timedelta, timezone


async def _admin_headers(client):
    resp = await client.post("/api/auth/login", json={"username": "admin", "password": "adminpass1"})
    return {"Authorization": f"Bearer {resp.json()['access_token']}"}


def _payload(name="Test Window", monitor_ids=None):
    now = datetime.now(timezone.utc)
    return {
        "name": name,
        "starts_at": (now + timedelta(hours=1)).isoformat(),
        "ends_at": (now + timedelta(hours=3)).isoformat(),
        "repeat_cron": None,
        "suppress_alerts": True,
        "monitor_ids": monitor_ids or [],
    }


class TestMaintenanceWindows:
    async def test_list_empty(self, client):
        headers = await _admin_headers(client)
        resp = await client.get("/api/maintenance", headers=headers)
        assert resp.status_code == 200
        assert resp.json() == []

    async def test_create_window(self, client):
        headers = await _admin_headers(client)
        resp = await client.post("/api/maintenance", json=_payload("My Window"), headers=headers)
        assert resp.status_code == 201
        data = resp.json()
        assert data["name"] == "My Window"
        assert data["suppress_alerts"] is True
        assert "id" in data

    async def test_get_window(self, client):
        headers = await _admin_headers(client)
        created = (await client.post("/api/maintenance", json=_payload(), headers=headers)).json()
        resp = await client.get(f"/api/maintenance/{created['id']}", headers=headers)
        assert resp.status_code == 200
        assert resp.json()["id"] == created["id"]

    async def test_get_window_not_found(self, client):
        headers = await _admin_headers(client)
        resp = await client.get("/api/maintenance/99999", headers=headers)
        assert resp.status_code == 404

    async def test_update_window_name(self, client):
        headers = await _admin_headers(client)
        created = (await client.post("/api/maintenance", json=_payload("Old"), headers=headers)).json()
        resp = await client.put(f"/api/maintenance/{created['id']}", json={"name": "Updated"}, headers=headers)
        assert resp.status_code == 200
        assert resp.json()["name"] == "Updated"

    async def test_update_suppress_alerts(self, client):
        headers = await _admin_headers(client)
        created = (await client.post("/api/maintenance", json=_payload(), headers=headers)).json()
        resp = await client.put(
            f"/api/maintenance/{created['id']}",
            json={"suppress_alerts": False},
            headers=headers,
        )
        assert resp.status_code == 200
        assert resp.json()["suppress_alerts"] is False

    async def test_update_window_not_found(self, client):
        headers = await _admin_headers(client)
        resp = await client.put("/api/maintenance/99999", json={"name": "X"}, headers=headers)
        assert resp.status_code == 404

    async def test_delete_window(self, client):
        headers = await _admin_headers(client)
        created = (await client.post("/api/maintenance", json=_payload(), headers=headers)).json()
        resp = await client.delete(f"/api/maintenance/{created['id']}", headers=headers)
        assert resp.status_code == 204

    async def test_delete_window_not_found(self, client):
        headers = await _admin_headers(client)
        resp = await client.delete("/api/maintenance/99999", headers=headers)
        assert resp.status_code == 404

    async def test_list_returns_created_windows(self, client):
        headers = await _admin_headers(client)
        await client.post("/api/maintenance", json=_payload("Win A"), headers=headers)
        await client.post("/api/maintenance", json=_payload("Win B"), headers=headers)
        resp = await client.get("/api/maintenance", headers=headers)
        assert resp.status_code == 200
        names = [w["name"] for w in resp.json()]
        assert "Win A" in names
        assert "Win B" in names

    async def test_create_with_repeat_cron(self, client):
        headers = await _admin_headers(client)
        payload = _payload("Cron Window")
        payload["repeat_cron"] = "0 2 * * 0"
        resp = await client.post("/api/maintenance", json=payload, headers=headers)
        assert resp.status_code == 201
        assert resp.json()["repeat_cron"] == "0 2 * * 0"


# ---------------------------------------------------------------------------
# Direct router function tests — bypass ASGI transport for line coverage
# ---------------------------------------------------------------------------

class TestMaintenanceDirect:
    """Call router functions directly with real DB sessions for line coverage."""

    def _make_body(self, name="Window", monitor_ids=None):
        from app.schemas.maintenance import MaintenanceCreate
        now = datetime.now(timezone.utc)
        return MaintenanceCreate(
            name=name,
            starts_at=now + timedelta(hours=1),
            ends_at=now + timedelta(hours=3),
            suppress_alerts=True,
            monitor_ids=monitor_ids or [],
        )

    async def test_list_windows_empty(self, db, admin_user):
        from app.routers.maintenance import list_windows
        result = await list_windows(admin_user, db)
        assert result == []

    async def test_create_and_list(self, db, admin_user):
        from app.routers.maintenance import create_window, list_windows
        body = self._make_body("My Window")
        window = await create_window(body, admin_user, db)
        assert window.name == "My Window"
        assert window.suppress_alerts is True

        listing = await list_windows(admin_user, db)
        assert len(listing) == 1
        assert listing[0].id == window.id

    async def test_get_window(self, db, admin_user):
        from app.routers.maintenance import create_window, get_window
        body = self._make_body()
        created = await create_window(body, admin_user, db)
        got = await get_window(created.id, admin_user, db)
        assert got.id == created.id

    async def test_get_window_not_found(self, db, admin_user):
        from app.routers.maintenance import get_window
        from fastapi import HTTPException
        with pytest.raises(HTTPException) as exc_info:
            await get_window(99999, admin_user, db)
        assert exc_info.value.status_code == 404

    async def test_update_window_name(self, db, admin_user):
        from app.routers.maintenance import create_window, update_window
        from app.schemas.maintenance import MaintenanceUpdate
        created = await create_window(self._make_body("Old"), admin_user, db)
        updated = await update_window(created.id, MaintenanceUpdate(name="New"), admin_user, db)
        assert updated.name == "New"

    async def test_update_suppress_alerts(self, db, admin_user):
        from app.routers.maintenance import create_window, update_window
        from app.schemas.maintenance import MaintenanceUpdate
        created = await create_window(self._make_body(), admin_user, db)
        updated = await update_window(created.id, MaintenanceUpdate(suppress_alerts=False), admin_user, db)
        assert updated.suppress_alerts is False

    async def test_update_dates(self, db, admin_user):
        from app.routers.maintenance import create_window, update_window
        from app.schemas.maintenance import MaintenanceUpdate
        created = await create_window(self._make_body(), admin_user, db)
        new_start = datetime.now(timezone.utc) + timedelta(hours=2)
        new_end = datetime.now(timezone.utc) + timedelta(hours=4)
        updated = await update_window(created.id, MaintenanceUpdate(starts_at=new_start, ends_at=new_end), admin_user, db)
        assert updated.id == created.id

    async def test_update_cron(self, db, admin_user):
        from app.routers.maintenance import create_window, update_window
        from app.schemas.maintenance import MaintenanceUpdate
        created = await create_window(self._make_body(), admin_user, db)
        updated = await update_window(created.id, MaintenanceUpdate(repeat_cron="0 3 * * 1"), admin_user, db)
        assert updated.repeat_cron == "0 3 * * 1"

    async def test_update_window_not_found(self, db, admin_user):
        from app.routers.maintenance import update_window
        from app.schemas.maintenance import MaintenanceUpdate
        from fastapi import HTTPException
        with pytest.raises(HTTPException) as exc_info:
            await update_window(99999, MaintenanceUpdate(name="X"), admin_user, db)
        assert exc_info.value.status_code == 404

    async def test_delete_window(self, db, admin_user):
        from app.routers.maintenance import create_window, delete_window, list_windows
        created = await create_window(self._make_body(), admin_user, db)
        await delete_window(created.id, admin_user, db)
        listing = await list_windows(admin_user, db)
        assert len(listing) == 0

    async def test_delete_window_not_found(self, db, admin_user):
        from app.routers.maintenance import delete_window
        from fastapi import HTTPException
        with pytest.raises(HTTPException) as exc_info:
            await delete_window(99999, admin_user, db)
        assert exc_info.value.status_code == 404
