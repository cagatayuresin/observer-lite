"""HTTP/HTTPS monitor checker (GET, POST, HEAD).

Uses a single shared :class:`httpx.AsyncClient` instance with a generous
connection pool so that hundreds of concurrent checks don't exhaust OS
file descriptors.  SSL verification is **disabled** here because the
:mod:`~app.checkers.ssl_checker` module handles certificate validation
separately, giving more detailed expiry information.
"""

import json
import time

import httpx

from app.checkers.base import CheckResult
from app.checkers.matchers import matches_body, matches_status_code


_client: httpx.AsyncClient | None = None


def get_http_client() -> httpx.AsyncClient:
    """Return the shared async HTTP client, creating it if necessary.

    The client is lazily created on first use and reused for all subsequent
    checks.  Call :func:`close_http_client` during application shutdown to
    drain in-flight connections gracefully.

    Returns:
        A configured :class:`httpx.AsyncClient` instance.
    """
    global _client
    if _client is None or _client.is_closed:
        _client = httpx.AsyncClient(
            limits=httpx.Limits(max_connections=200, max_keepalive_connections=50),
            follow_redirects=True,
            verify=False,  # SSL validity is checked separately
        )
    return _client


async def close_http_client() -> None:
    """Close and discard the shared HTTP client.

    Called from the FastAPI lifespan shutdown hook.
    """
    global _client
    if _client and not _client.is_closed:
        await _client.aclose()
        _client = None


async def http_check(
    url: str,
    method: str,
    headers_json: str | None,
    body: str | None,
    expected_status_expr: str,
    expected_body_type: str | None,
    expected_body_value: str | None,
    timeout_seconds: int,
    response_time_warning_ms: int,
) -> CheckResult:
    """Perform a single HTTP check against *url*.

    Args:
        url: Target URL (must start with ``http://`` or ``https://``).
        method: HTTP method — ``"GET"``, ``"POST"``, or ``"HEAD"``.
        headers_json: JSON-encoded ``dict`` of extra request headers, or
            ``None``.
        body: Raw request body string for POST requests, or ``None``.
        expected_status_expr: Status code DSL expression (e.g. ``"2xx"``).
        expected_body_type: Body match mode — ``"contains"``, ``"equals"``,
            ``"not_equals"``, or ``None`` to skip body matching.
        expected_body_value: Value to compare the response body against.
        timeout_seconds: Maximum wait time before aborting the request.
        response_time_warning_ms: Threshold above which the result is
            ``"warning"`` rather than ``"up"``.

    Returns:
        A :class:`~app.checkers.base.CheckResult` with ``status`` set to
        ``"up"``, ``"warning"``, or ``"down"``.
    """
    try:
        headers = json.loads(headers_json) if headers_json else {}
    except Exception:
        headers = {}

    client = get_http_client()
    start = time.monotonic()
    try:
        response = await client.request(
            method=method.upper(),
            url=url,
            headers=headers,
            content=body.encode() if body else None,
            timeout=timeout_seconds,
        )
        elapsed_ms = int((time.monotonic() - start) * 1000)

        status_ok = matches_status_code(response.status_code, expected_status_expr)
        body_ok = matches_body(response.text, expected_body_type, expected_body_value)

        if not status_ok or not body_ok:
            reason = []
            if not status_ok:
                reason.append(f"status {response.status_code} did not match '{expected_status_expr}'")
            if not body_ok:
                reason.append(f"body did not match {expected_body_type}='{expected_body_value}'")
            return CheckResult(
                status="down",
                response_time_ms=elapsed_ms,
                status_code=response.status_code,
                error_message="; ".join(reason),
            )

        final_status = "warning" if elapsed_ms > response_time_warning_ms else "up"
        return CheckResult(
            status=final_status,
            response_time_ms=elapsed_ms,
            status_code=response.status_code,
        )

    except httpx.TimeoutException:
        elapsed_ms = int((time.monotonic() - start) * 1000)
        return CheckResult(status="down", response_time_ms=elapsed_ms, error_message="Connection timed out")
    except Exception as exc:
        elapsed_ms = int((time.monotonic() - start) * 1000)
        return CheckResult(status="down", response_time_ms=elapsed_ms, error_message=str(exc)[:255])
