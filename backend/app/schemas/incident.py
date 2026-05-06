from datetime import datetime

from pydantic import BaseModel


class IncidentOut(BaseModel):
    id: int
    monitor_id: int
    started_at: datetime
    resolved_at: datetime | None
    duration_seconds: int | None
    root_cause: str | None
    notification_sent: bool
    recovery_sent: bool
    acknowledged_by: int | None
    acknowledged_at: datetime | None

    model_config = {"from_attributes": True}
