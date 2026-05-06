"""Unit tests for the email notification service."""

import json
import pytest
from unittest.mock import AsyncMock, MagicMock, patch

from app.services.email_service import send_email
from app.services.email_service import test_email_channel as _test_email_channel


def _cfg(host="smtp.example.com", recipients=None, use_tls=False, password_enc=""):
    return json.dumps({
        "smtp_host": host,
        "smtp_port": 587,
        "smtp_user": "user@example.com",
        "smtp_password_enc": password_enc,
        "smtp_from": "noreply@example.com",
        "recipients": recipients or ["admin@example.com"],
        "use_tls": use_tls,
    })


class TestSendEmail:
    async def test_successful_send(self):
        with patch("app.services.email_service.aiosmtplib.send", new_callable=AsyncMock) as mock_send:
            mock_send.return_value = None
            result = await send_email(_cfg(), "Subject", "<p>Body</p>")
        assert result is True
        mock_send.assert_called_once()

    async def test_missing_host_returns_false(self):
        cfg = json.dumps({"smtp_host": "", "recipients": ["a@b.com"]})
        result = await send_email(cfg, "S", "B")
        assert result is False

    async def test_missing_recipients_returns_false(self):
        cfg = json.dumps({"smtp_host": "smtp.example.com", "recipients": []})
        result = await send_email(cfg, "S", "B")
        assert result is False

    async def test_smtp_exception_returns_false(self):
        with patch("app.services.email_service.aiosmtplib.send", new_callable=AsyncMock) as mock_send:
            mock_send.side_effect = Exception("Connection refused")
            result = await send_email(_cfg(), "S", "B")
        assert result is False

    async def test_encrypted_password_decrypted(self):
        from app.utils.crypto import encrypt
        enc_pw = encrypt("secret123")
        cfg = json.dumps({
            "smtp_host": "smtp.example.com",
            "smtp_port": 587,
            "smtp_user": "u",
            "smtp_password_enc": enc_pw,
            "smtp_from": "f@example.com",
            "recipients": ["a@example.com"],
            "use_tls": False,
        })
        with patch("app.services.email_service.aiosmtplib.send", new_callable=AsyncMock) as mock_send:
            mock_send.return_value = None
            result = await send_email(cfg, "S", "B")
        assert result is True
        # Verify password was passed to send (not the encrypted form)
        call_kwargs = mock_send.call_args
        assert call_kwargs.kwargs.get("password") == "secret123" or \
               (call_kwargs.args and "secret123" in str(call_kwargs.args))

    async def test_tls_flag_forwarded(self):
        cfg = _cfg(use_tls=True)
        with patch("app.services.email_service.aiosmtplib.send", new_callable=AsyncMock) as mock_send:
            mock_send.return_value = None
            result = await send_email(cfg, "S", "B")
        assert result is True
        call_kwargs = mock_send.call_args.kwargs
        assert call_kwargs.get("use_tls") is True
        assert call_kwargs.get("start_tls") is False

    async def test_starttls_when_not_tls(self):
        cfg = _cfg(use_tls=False)
        with patch("app.services.email_service.aiosmtplib.send", new_callable=AsyncMock) as mock_send:
            mock_send.return_value = None
            await send_email(cfg, "S", "B")
        call_kwargs = mock_send.call_args.kwargs
        assert call_kwargs.get("use_tls") is False
        assert call_kwargs.get("start_tls") is True

    async def test_invalid_json_config_returns_false(self):
        result = await send_email("not-json", "S", "B")
        assert result is False

    async def test_bad_password_enc_falls_back(self):
        """If decryption fails, the raw value is used as a fallback."""
        cfg = json.dumps({
            "smtp_host": "smtp.example.com",
            "smtp_port": 587,
            "smtp_user": "u",
            "smtp_password_enc": "not-encrypted-value",
            "smtp_from": "f@example.com",
            "recipients": ["a@example.com"],
            "use_tls": False,
        })
        with patch("app.services.email_service.aiosmtplib.send", new_callable=AsyncMock) as mock_send:
            mock_send.return_value = None
            result = await send_email(cfg, "S", "B")
        # Falls back to raw value — should still attempt send
        assert result is True


class TestTestEmailChannel:
    async def test_delegates_to_send_email(self):
        with patch("app.services.email_service.aiosmtplib.send", new_callable=AsyncMock) as mock_send:
            mock_send.return_value = None
            result = await _test_email_channel(_cfg())
        assert result is True
