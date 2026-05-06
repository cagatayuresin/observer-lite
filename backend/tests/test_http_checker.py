"""Unit tests for the HTTP checker with mocked httpx.AsyncClient."""

import pytest
from unittest.mock import AsyncMock, MagicMock, patch

from app.checkers.http_checker import http_check
import httpx


def _make_response(status_code: int, text: str = "", elapsed_ms: int = 100) -> MagicMock:
    """Build a mock httpx.Response."""
    resp = MagicMock()
    resp.status_code = status_code
    resp.text = text
    return resp


async def _run_check(
    url="http://example.com",
    method="GET",
    headers_json=None,
    body=None,
    expected_status_expr="2xx",
    expected_body_type=None,
    expected_body_value=None,
    timeout_seconds=10,
    response_time_warning_ms=2000,
    mock_response=None,
    raise_exception=None,
):
    with patch("app.checkers.http_checker.get_http_client") as mock_get_client:
        mock_client = AsyncMock()
        mock_get_client.return_value = mock_client
        if raise_exception:
            mock_client.request = AsyncMock(side_effect=raise_exception)
        else:
            mock_client.request = AsyncMock(return_value=mock_response or _make_response(200))

        return await http_check(
            url=url,
            method=method,
            headers_json=headers_json,
            body=body,
            expected_status_expr=expected_status_expr,
            expected_body_type=expected_body_type,
            expected_body_value=expected_body_value,
            timeout_seconds=timeout_seconds,
            response_time_warning_ms=response_time_warning_ms,
        )


class TestHttpChecker:
    async def test_200_up(self):
        result = await _run_check(mock_response=_make_response(200))
        assert result.status == "up"
        assert result.status_code == 200

    async def test_500_down(self):
        result = await _run_check(
            expected_status_expr="2xx",
            mock_response=_make_response(500),
        )
        assert result.status == "down"
        assert "500" in result.error_message

    async def test_404_with_expected_404(self):
        result = await _run_check(
            expected_status_expr="404",
            mock_response=_make_response(404),
        )
        assert result.status == "up"

    async def test_timeout_returns_down(self):
        result = await _run_check(raise_exception=httpx.TimeoutException("timed out"))
        assert result.status == "down"
        assert "timed out" in result.error_message.lower()

    async def test_connection_error_returns_down(self):
        result = await _run_check(raise_exception=Exception("connection refused"))
        assert result.status == "down"
        assert result.error_message is not None

    async def test_body_contains_match(self):
        result = await _run_check(
            mock_response=_make_response(200, text="hello world"),
            expected_body_type="contains",
            expected_body_value="world",
        )
        assert result.status == "up"

    async def test_body_contains_no_match(self):
        result = await _run_check(
            mock_response=_make_response(200, text="hello"),
            expected_body_type="contains",
            expected_body_value="world",
        )
        assert result.status == "down"
        assert "body" in result.error_message

    async def test_post_method(self):
        result = await _run_check(
            method="POST",
            body='{"key": "value"}',
            mock_response=_make_response(201),
            expected_status_expr="201",
        )
        assert result.status == "up"

    async def test_invalid_headers_json_ignored(self):
        result = await _run_check(
            headers_json="not-valid-json",
            mock_response=_make_response(200),
        )
        assert result.status == "up"

    async def test_response_time_recorded(self):
        result = await _run_check(mock_response=_make_response(200))
        assert result.response_time_ms is not None
        assert result.response_time_ms >= 0

    async def test_negated_status_expr_5xx(self):
        result = await _run_check(
            expected_status_expr="!5xx",
            mock_response=_make_response(200),
        )
        assert result.status == "up"

    async def test_negated_status_expr_5xx_fails_on_500(self):
        result = await _run_check(
            expected_status_expr="!5xx",
            mock_response=_make_response(500),
        )
        assert result.status == "down"
