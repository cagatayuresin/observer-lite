from datetime import datetime

from pydantic import BaseModel, field_validator

VALID_TYPES = {"http_get", "http_post", "http_head", "ping", "heartbeat"}
VALID_BODY_TYPES = {"contains", "equals", "not_equals", None}


def _validate_status_expr(v: str) -> str:
    """Validate status code DSL expression like 2xx, !5xx, 200|404."""
    from app.checkers.matchers import parse_status_expression
    try:
        parse_status_expression(v)
    except ValueError as e:
        raise ValueError(str(e))
    return v


class MonitorCreate(BaseModel):
    name: str
    url: str
    monitor_type: str = "http_get"
    group_id: int | None = None
    is_enabled: bool = True
    check_interval_seconds: int = 60
    timeout_seconds: int = 10
    retry_count: int = 3
    retry_interval_seconds: int = 20
    alert_cooldown_seconds: int = 1800
    request_headers: str | None = None   # JSON string
    request_body: str | None = None
    expected_status_codes: str = "2xx"
    expected_body_type: str | None = None
    expected_body_value: str | None = None
    response_time_warning_ms: int = 2000
    ssl_check_enabled: bool = True
    ssl_expiry_warning_days: int = 30
    alerts_enabled: bool = True
    heartbeat_grace_seconds: int = 60

    @field_validator("monitor_type")
    @classmethod
    def validate_type(cls, v: str) -> str:
        if v not in VALID_TYPES:
            raise ValueError(f"monitor_type must be one of {VALID_TYPES}")
        return v

    @field_validator("expected_status_codes")
    @classmethod
    def validate_status(cls, v: str) -> str:
        return _validate_status_expr(v)

    @field_validator("expected_body_type")
    @classmethod
    def validate_body_type(cls, v: str | None) -> str | None:
        if v not in VALID_BODY_TYPES:
            raise ValueError(f"expected_body_type must be one of {VALID_BODY_TYPES}")
        return v


class MonitorUpdate(MonitorCreate):
    name: str | None = None
    url: str | None = None
    monitor_type: str | None = None


class MonitorPatch(BaseModel):
    is_enabled: bool | None = None
    alerts_enabled: bool | None = None
    ssl_check_enabled: bool | None = None
    group_id: int | None = None


class MonitorOut(BaseModel):
    id: int
    name: str
    url: str
    monitor_type: str
    group_id: int | None
    is_enabled: bool
    check_interval_seconds: int
    timeout_seconds: int
    retry_count: int
    retry_interval_seconds: int
    alert_cooldown_seconds: int
    request_headers: str | None
    request_body: str | None
    expected_status_codes: str
    expected_body_type: str | None
    expected_body_value: str | None
    response_time_warning_ms: int
    ssl_check_enabled: bool
    ssl_expiry_warning_days: int
    alerts_enabled: bool
    heartbeat_token: str | None
    heartbeat_grace_seconds: int
    current_status: str
    last_checked_at: datetime | None
    last_response_time_ms: int | None
    consecutive_failures: int
    created_at: datetime
    updated_at: datetime

    model_config = {"from_attributes": True}


class MonitorBulkAction(BaseModel):
    action: str  # enable|disable|delete
    ids: list[int]
