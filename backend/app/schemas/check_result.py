from datetime import datetime

from pydantic import BaseModel


class CheckResultOut(BaseModel):
    id: int
    monitor_id: int
    checked_at: datetime
    status: str
    response_time_ms: int | None
    status_code: int | None
    is_ssl_valid: bool | None
    ssl_expiry_days: int | None
    error_message: str | None

    model_config = {"from_attributes": True}
