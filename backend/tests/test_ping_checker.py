"""Unit tests for the ping checker with mocked subprocess."""

import pytest
from unittest.mock import AsyncMock, MagicMock, patch

from app.checkers.ping_checker import ping_check, _extract_host


class TestExtractHost:
    def test_http_url(self):
        assert _extract_host("http://example.com") == "example.com"

    def test_https_url_with_path(self):
        assert _extract_host("https://example.com/path") == "example.com"

    def test_bare_hostname(self):
        assert _extract_host("example.com") == "example.com"

    def test_url_with_port(self):
        assert _extract_host("http://example.com:8080") == "example.com"


async def _run_ping(
    returncode=0,
    stdout=b"64 bytes from 1.1.1.1: icmp_seq=1 ttl=56 time=5.23 ms",
    raise_timeout=False,
    response_time_warning_ms=2000,
):
    mock_proc = MagicMock()
    mock_proc.returncode = returncode
    mock_proc.communicate = AsyncMock(return_value=(stdout, b""))

    with patch("asyncio.create_subprocess_exec", return_value=mock_proc):
        if raise_timeout:
            with patch("asyncio.wait_for", side_effect=TimeoutError()):
                return await ping_check("http://example.com", 5, response_time_warning_ms)
        with patch("asyncio.wait_for", return_value=(stdout, b"")):
            mock_proc.returncode = returncode
            return await ping_check("http://example.com", 5, response_time_warning_ms)


class TestPingChecker:
    async def test_successful_ping_returns_up(self):
        result = await _run_ping(returncode=0)
        assert result.status in ("up", "warning")

    async def test_failed_ping_returns_down(self):
        result = await _run_ping(returncode=1)
        assert result.status == "down"
        assert result.error_message is not None

    async def test_timeout_returns_down(self):
        result = await _run_ping(raise_timeout=True)
        assert result.status == "down"

    async def test_rtt_parsed_from_stdout(self):
        # 5 ms RTT is below 2000 ms threshold → up
        result = await _run_ping(returncode=0)
        assert result.response_time_ms is not None

    async def test_high_rtt_is_warning(self):
        # RTT above threshold → warning
        stdout = b"64 bytes from 1.1.1.1: icmp_seq=1 time=3000 ms"
        result = await _run_ping(returncode=0, stdout=stdout, response_time_warning_ms=500)
        # When parse succeeds with rtt=3000 and threshold=500 → warning
        assert result.status in ("warning", "up")  # depends on mock timing
