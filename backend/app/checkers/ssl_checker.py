"""SSL certificate validity and expiry checker.

Runs a TLS handshake in a thread pool to avoid blocking the event loop,
then inspects the peer certificate's ``notAfter`` field to determine how
many days remain until expiry.
"""

import asyncio
import socket
import ssl
from datetime import datetime, timezone
from urllib.parse import urlparse


def _check_ssl_sync(hostname: str, port: int = 443) -> tuple[bool, int | None, str | None]:
    """Connect to *hostname:port* and inspect the TLS certificate (blocking).

    Args:
        hostname: Server hostname for SNI and certificate verification.
        port: TCP port to connect to (default 443).

    Returns:
        A 3-tuple ``(is_valid, days_until_expiry, error_message)``.
        *is_valid* is ``False`` when the certificate fails verification.
        *days_until_expiry* is ``None`` if the expiry date could not be
        parsed.  *error_message* is ``None`` on success.
    """
    ctx = ssl.create_default_context()
    try:
        with socket.create_connection((hostname, port), timeout=10) as sock:
            with ctx.wrap_socket(sock, server_hostname=hostname) as ssock:
                cert = ssock.getpeercert()
                not_after = cert.get("notAfter")
                if not_after:
                    expire_dt = datetime.strptime(not_after, "%b %d %H:%M:%S %Y %Z").replace(tzinfo=timezone.utc)
                    days = (expire_dt - datetime.now(timezone.utc)).days
                    return True, days, None
                return True, None, None
    except ssl.SSLCertVerificationError as e:
        return False, None, str(e)
    except Exception as e:
        return False, None, str(e)


async def ssl_check(url: str) -> tuple[bool, int | None, str | None]:
    """Async wrapper that dispatches :func:`_check_ssl_sync` to a thread pool.

    Args:
        url: Full URL — only the hostname and port are used.

    Returns:
        See :func:`_check_ssl_sync`.
    """
    parsed = urlparse(url)
    hostname = parsed.hostname or url
    port = parsed.port or 443
    return await asyncio.to_thread(_check_ssl_sync, hostname, port)
