"""Shared data structures used by all checker implementations."""

from dataclasses import dataclass
from datetime import datetime, timezone


@dataclass
class CheckResult:
    """The outcome of a single monitor probe.

    Produced by every checker (HTTP, Ping, SSL) and consumed by
    :func:`~app.services.check_service.process_result`, which persists
    the result and drives the incident state machine.

    Attributes:
        status: Probe outcome — ``"up"``, ``"down"``, or ``"warning"``.
        response_time_ms: Round-trip time in milliseconds, or ``None`` if
            the request failed before a response was received.
        status_code: HTTP status code returned by the server, or ``None``
            for non-HTTP checkers.
        is_ssl_valid: Whether the TLS certificate passed verification.
            ``None`` when SSL was not checked.
        ssl_expiry_days: Days until the certificate expires.  ``None`` when
            SSL was not checked or could not be parsed.
        error_message: Human-readable failure reason, truncated to 255 chars.
        checked_at: UTC timestamp set automatically if not provided.
    """

    status: str  # up|down|warning
    response_time_ms: int | None = None
    status_code: int | None = None
    is_ssl_valid: bool | None = None
    ssl_expiry_days: int | None = None
    error_message: str | None = None
    checked_at: datetime | None = None

    def __post_init__(self):
        if self.checked_at is None:
            self.checked_at = datetime.now(timezone.utc)
