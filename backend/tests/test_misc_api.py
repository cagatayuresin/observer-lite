"""Integration tests for groups, heartbeat, notifications, settings, and API keys endpoints."""

import pytest
from datetime import datetime, timezone
from unittest.mock import AsyncMock, patch


async def _admin_headers(client):
    resp = await client.post("/api/auth/login", json={"username": "admin", "password": "adminpass1"})
    return {"Authorization": f"Bearer {resp.json()['access_token']}"}


# ── Monitor Groups ────────────────────────────────────────────────────────────

class TestGroups:
    async def test_list_empty(self, client):
        headers = await _admin_headers(client)
        resp = await client.get("/api/groups", headers=headers)
        assert resp.status_code == 200
        assert resp.json() == []

    async def test_create_group(self, client):
        headers = await _admin_headers(client)
        resp = await client.post("/api/groups", json={"name": "Production"}, headers=headers)
        assert resp.status_code == 201
        assert resp.json()["name"] == "Production"

    async def test_update_group(self, client):
        headers = await _admin_headers(client)
        created = (await client.post("/api/groups", json={"name": "Old"}, headers=headers)).json()
        resp = await client.put(f"/api/groups/{created['id']}", json={"name": "New"}, headers=headers)
        assert resp.status_code == 200
        assert resp.json()["name"] == "New"

    async def test_delete_group(self, client):
        headers = await _admin_headers(client)
        created = (await client.post("/api/groups", json={"name": "Del"}, headers=headers)).json()
        resp = await client.delete(f"/api/groups/{created['id']}", headers=headers)
        assert resp.status_code == 204


# ── Notification Channels ─────────────────────────────────────────────────────

class TestNotificationChannels:
    def _email_cfg(self):
        return {
            "smtp_host": "smtp.example.com",
            "smtp_port": 587,
            "smtp_user": "u",
            "smtp_password_enc": "",
            "smtp_from": "noreply@example.com",
            "recipients": ["admin@example.com"],
            "use_tls": False,
        }

    async def test_list_channels(self, client):
        headers = await _admin_headers(client)
        resp = await client.get("/api/channels", headers=headers)
        assert resp.status_code == 200

    async def test_create_email_channel(self, client):
        headers = await _admin_headers(client)
        resp = await client.post("/api/channels", json={
            "name": "email-ch",
            "channel_type": "email",
            "config": self._email_cfg(),
            "is_enabled": True,
        }, headers=headers)
        assert resp.status_code == 201
        assert resp.json()["name"] == "email-ch"

    async def test_create_telegram_channel(self, client):
        headers = await _admin_headers(client)
        resp = await client.post("/api/channels", json={
            "name": "tg-ch",
            "channel_type": "telegram",
            "config": {"bot_token": "tok123", "chat_id": "99999"},
            "is_enabled": True,
        }, headers=headers)
        assert resp.status_code == 201

    async def test_get_channel(self, client):
        headers = await _admin_headers(client)
        created = (await client.post("/api/channels", json={
            "name": "x", "channel_type": "email", "config": self._email_cfg(), "is_enabled": True
        }, headers=headers)).json()
        resp = await client.get(f"/api/channels/{created['id']}", headers=headers)
        assert resp.status_code == 200

    async def test_update_channel(self, client):
        headers = await _admin_headers(client)
        created = (await client.post("/api/channels", json={
            "name": "old", "channel_type": "email", "config": self._email_cfg(), "is_enabled": True
        }, headers=headers)).json()
        resp = await client.put(f"/api/channels/{created['id']}", json={
            "name": "new", "config": self._email_cfg(), "is_enabled": False,
        }, headers=headers)
        assert resp.status_code == 200
        assert resp.json()["name"] == "new"

    async def test_delete_channel(self, client):
        headers = await _admin_headers(client)
        created = (await client.post("/api/channels", json={
            "name": "del", "channel_type": "email", "config": self._email_cfg(), "is_enabled": True
        }, headers=headers)).json()
        resp = await client.delete(f"/api/channels/{created['id']}", headers=headers)
        assert resp.status_code == 204

    async def test_test_channel_email(self, client):
        headers = await _admin_headers(client)
        created = (await client.post("/api/channels", json={
            "name": "testable", "channel_type": "email", "config": self._email_cfg(), "is_enabled": True
        }, headers=headers)).json()
        with patch("app.routers.notifications.test_email_channel", new_callable=AsyncMock) as mock_test:
            mock_test.return_value = True
            resp = await client.post(f"/api/channels/{created['id']}/test", headers=headers)
        assert resp.status_code == 200

    async def test_test_channel_failure(self, client):
        headers = await _admin_headers(client)
        created = (await client.post("/api/channels", json={
            "name": "bad", "channel_type": "email", "config": self._email_cfg(), "is_enabled": True
        }, headers=headers)).json()
        with patch("app.routers.notifications.test_email_channel", new_callable=AsyncMock) as mock_test:
            mock_test.return_value = False
            resp = await client.post(f"/api/channels/{created['id']}/test", headers=headers)
        assert resp.status_code == 502


# ── Heartbeat ─────────────────────────────────────────────────────────────────

class TestHeartbeat:
    """Heartbeat uses AsyncSessionLocal directly, so we patch it."""

    async def _create_heartbeat_monitor(self, client, headers):
        resp = await client.post("/api/monitors", json={
            "name": "HB Monitor",
            "url": "heartbeat://myapp",
            "monitor_type": "heartbeat",
            "check_interval_seconds": 300,
            "timeout_seconds": 10,
            "retry_count": 1,
            "expected_status_codes": "2xx",
        }, headers=headers)
        return resp.json()

    async def test_heartbeat_get(self, client, engine):
        from sqlalchemy.ext.asyncio import async_sessionmaker
        factory = async_sessionmaker(engine, expire_on_commit=False)

        headers = await _admin_headers(client)
        monitor = await self._create_heartbeat_monitor(client, headers)
        token = monitor["heartbeat_token"]

        with patch("app.routers.heartbeat.AsyncSessionLocal", factory):
            resp = await client.get(f"/api/heartbeat/{token}")
        assert resp.status_code == 200

    async def test_heartbeat_post(self, client, engine):
        from sqlalchemy.ext.asyncio import async_sessionmaker
        factory = async_sessionmaker(engine, expire_on_commit=False)

        headers = await _admin_headers(client)
        monitor = await self._create_heartbeat_monitor(client, headers)
        token = monitor["heartbeat_token"]

        with patch("app.routers.heartbeat.AsyncSessionLocal", factory):
            resp = await client.post(f"/api/heartbeat/{token}")
        assert resp.status_code == 200

    async def test_heartbeat_invalid_token(self, client, engine):
        from sqlalchemy.ext.asyncio import async_sessionmaker
        factory = async_sessionmaker(engine, expire_on_commit=False)
        with patch("app.routers.heartbeat.AsyncSessionLocal", factory):
            resp = await client.get("/api/heartbeat/invalid-token-xyz")
        assert resp.status_code == 404


# ── Settings ─────────────────────────────────────────────────────────────────

class TestSettings:
    async def test_get_settings(self, client):
        headers = await _admin_headers(client)
        resp = await client.get("/api/settings", headers=headers)
        assert resp.status_code == 200

    async def test_update_settings(self, client):
        headers = await _admin_headers(client)
        resp = await client.put("/api/settings", json={"data_retention_days": "60"}, headers=headers)
        assert resp.status_code == 200


# ── API Keys ──────────────────────────────────────────────────────────────────

class TestApiKeys:
    async def test_list_api_keys(self, client):
        headers = await _admin_headers(client)
        resp = await client.get("/api/api-keys", headers=headers)
        assert resp.status_code == 200
        assert isinstance(resp.json(), list)

    async def test_create_api_key(self, client):
        headers = await _admin_headers(client)
        resp = await client.post("/api/api-keys", json={"name": "my-key"}, headers=headers)
        assert resp.status_code == 201
        data = resp.json()
        assert "raw_key" in data
        assert data["raw_key"].startswith("obs_")
        assert "key_prefix" in data

    async def test_delete_api_key(self, client):
        headers = await _admin_headers(client)
        created = (await client.post("/api/api-keys", json={"name": "del-key"}, headers=headers)).json()
        resp = await client.delete(f"/api/api-keys/{created['id']}", headers=headers)
        assert resp.status_code == 204

    async def test_api_key_authenticates_request(self, client):
        """Create a key, then use it as Bearer token to call an endpoint."""
        headers = await _admin_headers(client)
        created = (await client.post("/api/api-keys", json={"name": "auth-key"}, headers=headers)).json()
        raw_key = created["raw_key"]
        resp = await client.get("/api/auth/me", headers={"Authorization": f"Bearer {raw_key}"})
        assert resp.status_code == 200

    async def test_invalid_api_key_returns_401(self, client):
        resp = await client.get("/api/auth/me", headers={"Authorization": "Bearer obs_invalidkey123"})
        assert resp.status_code == 401

    async def test_wrong_jwt_type_returns_401(self, client):
        """A refresh token should not be accepted as an access token."""
        from jose import jwt as _jwt
        from app.config import get_settings
        import time
        cfg = get_settings()
        payload = {"sub": "1", "role": "superadmin", "type": "refresh", "exp": int(time.time()) + 3600}
        token = _jwt.encode(payload, cfg.secret_key, algorithm="HS256")
        resp = await client.get("/api/auth/me", headers={"Authorization": f"Bearer {token}"})
        assert resp.status_code == 401


# ── Audit Log ─────────────────────────────────────────────────────────────────

class TestAuditLog:
    async def test_list_audit_log(self, client):
        headers = await _admin_headers(client)
        resp = await client.get("/api/audit-log", headers=headers)
        assert resp.status_code == 200
        assert isinstance(resp.json(), list)


# ── Pagination ────────────────────────────────────────────────────────────────

class TestPagination:
    def test_offset_calculation(self):
        from app.utils.pagination import PaginationParams
        p = PaginationParams(page=3, per_page=20)
        assert p.offset == 40
        assert p.limit == 20

    def test_first_page(self):
        from app.utils.pagination import PaginationParams
        p = PaginationParams(page=1, per_page=50)
        assert p.offset == 0
