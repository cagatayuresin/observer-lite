"""Integration tests for /api/users/* endpoints (superadmin only)."""

import pytest


async def _admin_headers(client):
    resp = await client.post("/api/auth/login", json={"username": "admin", "password": "adminpass1"})
    return {"Authorization": f"Bearer {resp.json()['access_token']}"}


class TestListUsers:
    async def test_list_users(self, client):
        headers = await _admin_headers(client)
        resp = await client.get("/api/users", headers=headers)
        assert resp.status_code == 200
        assert isinstance(resp.json(), list)
        assert len(resp.json()) >= 1


class TestCreateUser:
    async def test_create_user(self, client):
        headers = await _admin_headers(client)
        resp = await client.post("/api/users", json={
            "username": "newuser",
            "email": "newuser@example.com",
            "password": "password123",
            "role": "viewer",
        }, headers=headers)
        assert resp.status_code == 201
        data = resp.json()
        assert data["username"] == "newuser"
        assert data["role"] == "viewer"
        assert data["force_pw_change"] is True

    async def test_duplicate_username(self, client):
        headers = await _admin_headers(client)
        payload = {"username": "dup", "email": "dup@example.com", "password": "pass1234", "role": "viewer"}
        await client.post("/api/users", json=payload, headers=headers)
        resp = await client.post("/api/users", json=payload, headers=headers)
        assert resp.status_code == 409


class TestGetUser:
    async def test_get_existing(self, client):
        headers = await _admin_headers(client)
        created = (await client.post("/api/users", json={
            "username": "getu", "email": "getu@example.com", "password": "pass1234", "role": "viewer"
        }, headers=headers)).json()
        resp = await client.get(f"/api/users/{created['id']}", headers=headers)
        assert resp.status_code == 200

    async def test_get_not_found(self, client):
        headers = await _admin_headers(client)
        resp = await client.get("/api/users/99999", headers=headers)
        assert resp.status_code == 404


class TestUpdateUser:
    async def test_update_role(self, client):
        headers = await _admin_headers(client)
        created = (await client.post("/api/users", json={
            "username": "updu", "email": "updu@example.com", "password": "pass1234", "role": "viewer"
        }, headers=headers)).json()
        resp = await client.put(f"/api/users/{created['id']}", json={"role": "admin"}, headers=headers)
        assert resp.status_code == 200
        assert resp.json()["role"] == "admin"

    async def test_deactivate_user(self, client):
        headers = await _admin_headers(client)
        created = (await client.post("/api/users", json={
            "username": "deact", "email": "deact@example.com", "password": "pass1234", "role": "viewer"
        }, headers=headers)).json()
        resp = await client.put(f"/api/users/{created['id']}", json={"is_active": False}, headers=headers)
        assert resp.status_code == 200
        assert resp.json()["is_active"] is False


class TestDeleteUser:
    async def test_delete_user(self, client):
        headers = await _admin_headers(client)
        created = (await client.post("/api/users", json={
            "username": "delu", "email": "delu@example.com", "password": "pass1234", "role": "viewer"
        }, headers=headers)).json()
        resp = await client.delete(f"/api/users/{created['id']}", headers=headers)
        assert resp.status_code == 204

    async def test_cannot_delete_self(self, client):
        # Login as admin, get own ID
        login = await client.post("/api/auth/login", json={"username": "admin", "password": "adminpass1"})
        token = login.json()["access_token"]
        headers = {"Authorization": f"Bearer {token}"}
        me = (await client.get("/api/auth/me", headers=headers)).json()
        resp = await client.delete(f"/api/users/{me['id']}", headers=headers)
        assert resp.status_code == 400


class TestResetPassword:
    async def test_reset_password(self, client):
        headers = await _admin_headers(client)
        created = (await client.post("/api/users", json={
            "username": "resetme", "email": "resetme@example.com", "password": "pass1234", "role": "viewer"
        }, headers=headers)).json()
        resp = await client.post(f"/api/users/{created['id']}/reset-password", headers=headers)
        assert resp.status_code == 204
