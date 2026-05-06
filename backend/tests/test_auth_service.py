"""Unit tests for auth_service: password hashing and JWT operations."""

import pytest
from jose import JWTError

from app.services.auth_service import (
    create_access_token,
    create_refresh_token,
    decode_token,
    hash_password,
    verify_password,
)


class TestPasswordHashing:
    def test_hash_is_not_plaintext(self):
        pw = "hunter2"
        assert hash_password(pw) != pw

    def test_verify_correct_password(self):
        pw = "correcthorsebatterystaple"
        assert verify_password(pw, hash_password(pw)) is True

    def test_verify_wrong_password(self):
        assert verify_password("wrong", hash_password("right")) is False

    def test_different_hashes_same_password(self):
        pw = "same"
        assert hash_password(pw) != hash_password(pw)  # different salts


class TestJWT:
    def test_access_token_decode(self):
        token = create_access_token(42, "admin")
        payload = decode_token(token)
        assert payload["sub"] == "42"
        assert payload["role"] == "admin"
        assert payload["type"] == "access"

    def test_refresh_token_decode(self):
        token = create_refresh_token(7)
        payload = decode_token(token)
        assert payload["sub"] == "7"
        assert payload["type"] == "refresh"
        assert "role" not in payload

    def test_invalid_token_raises(self):
        with pytest.raises(JWTError):
            decode_token("not.a.valid.token")

    def test_tampered_token_raises(self):
        token = create_access_token(1, "viewer")
        tampered = token[:-5] + "XXXXX"
        with pytest.raises(JWTError):
            decode_token(tampered)

    def test_access_token_contains_exp(self):
        payload = decode_token(create_access_token(1, "admin"))
        assert "exp" in payload

    def test_refresh_token_has_longer_expiry(self):
        access_payload = decode_token(create_access_token(1, "admin"))
        refresh_payload = decode_token(create_refresh_token(1))
        assert refresh_payload["exp"] > access_payload["exp"]
