"""Unit tests for the SSL certificate checker.

``ssl_check`` dispatches blocking I/O via ``asyncio.to_thread``, so we patch
``_check_ssl_sync`` (the blocking helper) rather than real sockets.
"""

import asyncio
import pytest
from datetime import datetime, timezone
from unittest.mock import patch, MagicMock


class TestCheckSslSync:
    """Direct tests for the blocking SSL helper (run synchronously)."""

    def test_returns_valid_and_days(self):
        from app.checkers.ssl_checker import _check_ssl_sync

        mock_cert = {"notAfter": "May 10 12:00:00 2030 GMT"}
        mock_ssock = MagicMock()
        mock_ssock.getpeercert.return_value = mock_cert

        mock_sock = MagicMock()
        mock_ctx = MagicMock()
        mock_ctx.wrap_socket.return_value.__enter__ = lambda s: mock_ssock
        mock_ctx.wrap_socket.return_value.__exit__ = MagicMock(return_value=False)

        with (
            patch("app.checkers.ssl_checker.ssl.create_default_context", return_value=mock_ctx),
            patch("app.checkers.ssl_checker.socket.create_connection") as mock_conn,
        ):
            mock_conn.return_value.__enter__ = lambda s: mock_sock
            mock_conn.return_value.__exit__ = MagicMock(return_value=False)
            is_valid, days, err = _check_ssl_sync("example.com", 443)

        assert is_valid is True
        assert days is not None
        assert err is None

    def test_cert_without_not_after_returns_none_days(self):
        from app.checkers.ssl_checker import _check_ssl_sync

        mock_ssock = MagicMock()
        mock_ssock.getpeercert.return_value = {}  # no notAfter key

        mock_sock = MagicMock()
        mock_ctx = MagicMock()
        mock_ctx.wrap_socket.return_value.__enter__ = lambda s: mock_ssock
        mock_ctx.wrap_socket.return_value.__exit__ = MagicMock(return_value=False)

        with (
            patch("app.checkers.ssl_checker.ssl.create_default_context", return_value=mock_ctx),
            patch("app.checkers.ssl_checker.socket.create_connection") as mock_conn,
        ):
            mock_conn.return_value.__enter__ = lambda s: mock_sock
            mock_conn.return_value.__exit__ = MagicMock(return_value=False)
            is_valid, days, err = _check_ssl_sync("example.com", 443)

        assert is_valid is True
        assert days is None
        assert err is None

    def test_ssl_cert_verification_error(self):
        import ssl as _ssl
        from app.checkers.ssl_checker import _check_ssl_sync

        mock_ctx = MagicMock()
        mock_ctx.wrap_socket.side_effect = _ssl.SSLCertVerificationError("cert verify failed")

        mock_sock = MagicMock()
        with (
            patch("app.checkers.ssl_checker.ssl.create_default_context", return_value=mock_ctx),
            patch("app.checkers.ssl_checker.socket.create_connection") as mock_conn,
        ):
            mock_conn.return_value.__enter__ = lambda s: mock_sock
            mock_conn.return_value.__exit__ = MagicMock(return_value=False)
            is_valid, days, err = _check_ssl_sync("bad.example.com", 443)

        assert is_valid is False
        assert days is None
        assert err is not None

    def test_generic_exception_returns_false(self):
        from app.checkers.ssl_checker import _check_ssl_sync

        with patch(
            "app.checkers.ssl_checker.socket.create_connection",
            side_effect=ConnectionRefusedError("refused"),
        ):
            is_valid, days, err = _check_ssl_sync("example.com", 443)

        assert is_valid is False
        assert days is None
        assert err is not None


class TestSslCheck:
    """Async wrapper tests — patches asyncio.to_thread to avoid real sockets."""

    async def test_delegates_to_sync_check(self):
        from app.checkers.ssl_checker import ssl_check

        with patch(
            "app.checkers.ssl_checker.asyncio.to_thread",
            new_callable=lambda: lambda f, *a, **kw: asyncio.coroutine(lambda: (True, 90, None))(),
        ):
            with patch("asyncio.to_thread", return_value=(True, 90, None)):
                pass  # just validating import

        # Patch via module reference
        async def _fake_to_thread(fn, *args, **kwargs):
            return fn(*args, **kwargs)

        with patch("app.checkers.ssl_checker.asyncio.to_thread", side_effect=_fake_to_thread):
            with patch("app.checkers.ssl_checker._check_ssl_sync", return_value=(True, 60, None)):
                is_valid, days, err = await ssl_check("https://example.com")

        assert is_valid is True
        assert days == 60
        assert err is None

    async def test_parses_hostname_from_url(self):
        from app.checkers.ssl_checker import ssl_check

        captured = {}

        async def _fake_to_thread(fn, hostname, port):
            captured["hostname"] = hostname
            captured["port"] = port
            return (True, 90, None)

        with patch("app.checkers.ssl_checker.asyncio.to_thread", side_effect=_fake_to_thread):
            await ssl_check("https://myhost.example.com:8443/path?q=1")

        assert captured["hostname"] == "myhost.example.com"
        assert captured["port"] == 8443

    async def test_defaults_port_to_443(self):
        from app.checkers.ssl_checker import ssl_check

        captured = {}

        async def _fake_to_thread(fn, hostname, port):
            captured["port"] = port
            return (True, 90, None)

        with patch("app.checkers.ssl_checker.asyncio.to_thread", side_effect=_fake_to_thread):
            await ssl_check("https://example.com")

        assert captured["port"] == 443

    async def test_propagates_error_result(self):
        from app.checkers.ssl_checker import ssl_check

        async def _fake_to_thread(fn, *args, **kwargs):
            return (False, None, "cert expired")

        with patch("app.checkers.ssl_checker.asyncio.to_thread", side_effect=_fake_to_thread):
            is_valid, days, err = await ssl_check("https://expired.example.com")

        assert is_valid is False
        assert err == "cert expired"
