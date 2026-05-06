"""Integration tests for /api/monitors/* endpoints."""



async def _login(client):
    resp = await client.post("/api/auth/login", json={"username": "admin", "password": "adminpass1"})
    return resp.json()["access_token"]


async def _auth_headers(client):
    return {"Authorization": f"Bearer {await _login(client)}"}


async def _create_monitor(client, headers, **overrides):
    payload = {
        "name": "Test Monitor",
        "url": "https://example.com",
        "monitor_type": "http_get",
        "check_interval_seconds": 60,
        "timeout_seconds": 10,
        "retry_count": 3,
        "expected_status_codes": "2xx",
        **overrides,
    }
    resp = await client.post("/api/monitors", json=payload, headers=headers)
    return resp


class TestListMonitors:
    async def test_empty_list(self, client):
        headers = await _auth_headers(client)
        resp = await client.get("/api/monitors", headers=headers)
        assert resp.status_code == 200
        assert resp.json() == []

    async def test_list_after_create(self, client):
        headers = await _auth_headers(client)
        await _create_monitor(client, headers)
        resp = await client.get("/api/monitors", headers=headers)
        assert resp.status_code == 200
        assert len(resp.json()) == 1

    async def test_unauthenticated(self, client):
        resp = await client.get("/api/monitors")
        assert resp.status_code == 401


class TestCreateMonitor:
    async def test_create_valid(self, client):
        headers = await _auth_headers(client)
        resp = await _create_monitor(client, headers)
        assert resp.status_code == 201
        data = resp.json()
        assert data["name"] == "Test Monitor"
        assert data["url"] == "https://example.com"
        assert "id" in data

    async def test_create_heartbeat_type(self, client):
        headers = await _auth_headers(client)
        resp = await _create_monitor(client, headers, name="HB", monitor_type="heartbeat", url="heartbeat://myapp")
        assert resp.status_code == 201
        data = resp.json()
        assert data["heartbeat_token"] is not None

    async def test_create_missing_name(self, client):
        headers = await _auth_headers(client)
        resp = await client.post("/api/monitors", json={"url": "https://x.com"}, headers=headers)
        assert resp.status_code == 422

    async def test_create_with_status_filter(self, client):
        headers = await _auth_headers(client)
        await _create_monitor(client, headers)
        resp = await client.get("/api/monitors?status=pending", headers=headers)
        assert resp.status_code == 200

    async def test_viewer_cannot_create(self, client):
        # Create viewer, log in, try to create monitor
        admin_headers = await _auth_headers(client)
        create_resp = await client.post("/api/users", json={
            "username": "viewer2",
            "email": "viewer2@example.com",
            "password": "viewpass123",
            "role": "viewer",
        }, headers=admin_headers)
        assert create_resp.status_code == 201, create_resp.text
        login = await client.post("/api/auth/login", json={"username": "viewer2", "password": "viewpass123"})
        assert login.status_code == 200, login.text
        viewer_headers = {"Authorization": f"Bearer {login.json()['access_token']}"}
        resp = await _create_monitor(client, viewer_headers)
        assert resp.status_code == 403


class TestGetMonitor:
    async def test_get_existing(self, client):
        headers = await _auth_headers(client)
        created = (await _create_monitor(client, headers)).json()
        resp = await client.get(f"/api/monitors/{created['id']}", headers=headers)
        assert resp.status_code == 200
        assert resp.json()["id"] == created["id"]

    async def test_get_not_found(self, client):
        headers = await _auth_headers(client)
        resp = await client.get("/api/monitors/99999", headers=headers)
        assert resp.status_code == 404


class TestUpdateMonitor:
    async def test_put_update(self, client):
        headers = await _auth_headers(client)
        created = (await _create_monitor(client, headers)).json()
        resp = await client.put(f"/api/monitors/{created['id']}", json={
            "name": "Updated",
            "url": "https://example.com",
            "monitor_type": "http_get",
            "check_interval_seconds": 120,
            "timeout_seconds": 10,
            "retry_count": 3,
            "expected_status_codes": "2xx",
        }, headers=headers)
        assert resp.status_code == 200
        assert resp.json()["name"] == "Updated"

    async def test_patch_update(self, client):
        headers = await _auth_headers(client)
        created = (await _create_monitor(client, headers)).json()
        resp = await client.patch(f"/api/monitors/{created['id']}", json={"alerts_enabled": False}, headers=headers)
        assert resp.status_code == 200
        assert resp.json()["alerts_enabled"] is False


class TestDeleteMonitor:
    async def test_delete(self, client):
        headers = await _auth_headers(client)
        created = (await _create_monitor(client, headers)).json()
        resp = await client.delete(f"/api/monitors/{created['id']}", headers=headers)
        assert resp.status_code == 204
        # Verify gone
        assert (await client.get(f"/api/monitors/{created['id']}", headers=headers)).status_code == 404

    async def test_delete_not_found(self, client):
        headers = await _auth_headers(client)
        resp = await client.delete("/api/monitors/99999", headers=headers)
        assert resp.status_code == 404


class TestMonitorLifecycle:
    async def test_pause(self, client):
        headers = await _auth_headers(client)
        created = (await _create_monitor(client, headers)).json()
        resp = await client.post(f"/api/monitors/{created['id']}/pause", headers=headers)
        assert resp.status_code == 204

    async def test_resume(self, client):
        headers = await _auth_headers(client)
        created = (await _create_monitor(client, headers)).json()
        await client.post(f"/api/monitors/{created['id']}/pause", headers=headers)
        resp = await client.post(f"/api/monitors/{created['id']}/resume", headers=headers)
        assert resp.status_code == 204

    async def test_check_now(self, client):
        headers = await _auth_headers(client)
        created = (await _create_monitor(client, headers)).json()
        from unittest.mock import patch, AsyncMock
        # Patch run_monitor_check so create_task receives a real coroutine
        with patch("app.routers.monitors.run_monitor_check", new_callable=AsyncMock):
            resp = await client.post(f"/api/monitors/{created['id']}/check-now", headers=headers)
        assert resp.status_code == 202


class TestBulkAction:
    async def test_bulk_disable(self, client):
        headers = await _auth_headers(client)
        m1 = (await _create_monitor(client, headers, name="M1")).json()
        m2 = (await _create_monitor(client, headers, name="M2")).json()
        resp = await client.post("/api/monitors/bulk", json={"action": "disable", "ids": [m1["id"], m2["id"]]}, headers=headers)
        assert resp.status_code == 204

    async def test_bulk_delete(self, client):
        headers = await _auth_headers(client)
        m = (await _create_monitor(client, headers, name="ToDelete")).json()
        resp = await client.post("/api/monitors/bulk", json={"action": "delete", "ids": [m["id"]]}, headers=headers)
        assert resp.status_code == 204


class TestMonitorStats:
    async def test_stats_endpoint(self, client):
        headers = await _auth_headers(client)
        created = (await _create_monitor(client, headers)).json()
        resp = await client.get(f"/api/monitors/{created['id']}/stats", headers=headers)
        assert resp.status_code == 200
        data = resp.json()
        assert "uptime_percent" in data
        assert "total_incidents" in data

    async def test_history_endpoint(self, client):
        headers = await _auth_headers(client)
        created = (await _create_monitor(client, headers)).json()
        resp = await client.get(f"/api/check-results?monitor_id={created['id']}", headers=headers)
        assert resp.status_code == 200
        assert isinstance(resp.json(), list)

    async def test_incidents_endpoint(self, client):
        headers = await _auth_headers(client)
        created = (await _create_monitor(client, headers)).json()
        resp = await client.get(f"/api/incidents?monitor_id={created['id']}", headers=headers)
        assert resp.status_code == 200
        assert isinstance(resp.json(), list)
