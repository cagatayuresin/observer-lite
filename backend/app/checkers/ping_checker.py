"""ICMP ping checker using the system ``ping`` binary.

Spawns ``ping -c 1 -W <timeout> <host>`` as a subprocess so that no raw
socket privileges are required inside the container (``iputils-ping`` is
installed in the Docker image).  RTT is parsed from the ping output rather
than wall-clock time for higher accuracy.
"""

import asyncio
import re
import time
from urllib.parse import urlparse

from app.checkers.base import CheckResult


def _extract_host(url: str) -> str:
    """Return the hostname portion of *url*, falling back to *url* itself."""
    parsed = urlparse(url)
    return parsed.hostname or url


async def ping_check(url: str, timeout_seconds: int, response_time_warning_ms: int) -> CheckResult:
    """Ping *url*'s hostname once and return a :class:`~app.checkers.base.CheckResult`.

    Args:
        url: Monitor URL — only the hostname component is used.
        timeout_seconds: ``-W`` argument passed to the ``ping`` binary.
        response_time_warning_ms: RTT threshold for a ``"warning"`` result.

    Returns:
        ``"up"`` / ``"warning"`` on success, ``"down"`` on failure or timeout.
    """
    host = _extract_host(url)
    start = time.monotonic()
    try:
        proc = await asyncio.create_subprocess_exec(
            "ping", "-c", "1", "-W", str(timeout_seconds), host,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        stdout, _ = await asyncio.wait_for(proc.communicate(), timeout=timeout_seconds + 2)
        elapsed_ms = int((time.monotonic() - start) * 1000)

        if proc.returncode != 0:
            return CheckResult(status="down", response_time_ms=elapsed_ms, error_message=f"Ping failed for {host}")

        # Parse RTT from ping output
        rtt_ms = elapsed_ms
        match = re.search(r"time[=<]([\d.]+)\s*ms", stdout.decode(), re.IGNORECASE)
        if match:
            rtt_ms = int(float(match.group(1)))

        final_status = "warning" if rtt_ms > response_time_warning_ms else "up"
        return CheckResult(status=final_status, response_time_ms=rtt_ms)

    except TimeoutError:
        elapsed_ms = int((time.monotonic() - start) * 1000)
        return CheckResult(status="down", response_time_ms=elapsed_ms, error_message="Ping timed out")
    except Exception as exc:
        elapsed_ms = int((time.monotonic() - start) * 1000)
        return CheckResult(status="down", response_time_ms=elapsed_ms, error_message=str(exc)[:255])
