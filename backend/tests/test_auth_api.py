"""Integration tests for /api/auth/* endpoints."""

import pytest


class TestLogin:
    async def test_valid_login(self, client):
        resp = await client.post("/api/auth/login", json={"username": "admin", "password": "adminpass1"})
        assert resp.status_code == 200
        data = resp.json()
        assert "access_token" in data
        assert "refresh_token" in data
        assert data["token_type"] == "bearer"

    async def test_wrong_password(self, client):
        resp = await client.post("/api/auth/login", json={"username": "admin", "password": "wrong"})
        assert resp.status_code == 401

    async def test_unknown_user(self, client):
        resp = await client.post("/api/auth/login", json={"username": "nobody", "password": "pw"})
        assert resp.status_code == 401

    async def test_missing_fields(self, client):
        resp = await client.post("/api/auth/login", json={"username": "admin"})
        assert resp.status_code == 422


class TestMe:
    async def test_me_with_valid_token(self, client):
        login = await client.post("/api/auth/login", json={"username": "admin", "password": "adminpass1"})
        token = login.json()["access_token"]
        resp = await client.get("/api/auth/me", headers={"Authorization": f"Bearer {token}"})
        assert resp.status_code == 200
        assert resp.json()["username"] == "admin"

    async def test_me_without_token(self, client):
        resp = await client.get("/api/auth/me")
        assert resp.status_code == 401

    async def test_me_with_invalid_token(self, client):
        resp = await client.get("/api/auth/me", headers={"Authorization": "Bearer bad.token.here"})
        assert resp.status_code == 401


class TestRefresh:
    async def test_refresh_returns_new_tokens(self, client):
        login = await client.post("/api/auth/login", json={"username": "admin", "password": "adminpass1"})
        refresh_token = login.json()["refresh_token"]
        resp = await client.post("/api/auth/refresh", json={"refresh_token": refresh_token})
        assert resp.status_code == 200
        assert "access_token" in resp.json()

    async def test_refresh_with_access_token_fails(self, client):
        login = await client.post("/api/auth/login", json={"username": "admin", "password": "adminpass1"})
        access_token = login.json()["access_token"]
        resp = await client.post("/api/auth/refresh", json={"refresh_token": access_token})
        assert resp.status_code == 401


class TestChangePassword:
    async def _get_token(self, client):
        login = await client.post("/api/auth/login", json={"username": "admin", "password": "adminpass1"})
        return login.json()["access_token"]

    async def test_change_password_success(self, client):
        token = await self._get_token(client)
        resp = await client.post(
            "/api/auth/change-password",
            json={"current_password": "adminpass1", "new_password": "newpassword99"},
            headers={"Authorization": f"Bearer {token}"},
        )
        assert resp.status_code == 204

    async def test_wrong_current_password(self, client):
        token = await self._get_token(client)
        resp = await client.post(
            "/api/auth/change-password",
            json={"current_password": "wrongcurrent", "new_password": "newpassword99"},
            headers={"Authorization": f"Bearer {token}"},
        )
        assert resp.status_code == 400

    async def test_password_too_short(self, client):
        token = await self._get_token(client)
        resp = await client.post(
            "/api/auth/change-password",
            json={"current_password": "adminpass1", "new_password": "short"},
            headers={"Authorization": f"Bearer {token}"},
        )
        assert resp.status_code == 422

    async def test_unauthenticated_change(self, client):
        resp = await client.post(
            "/api/auth/change-password",
            json={"current_password": "adminpass1", "new_password": "newpassword99"},
        )
        assert resp.status_code == 401
