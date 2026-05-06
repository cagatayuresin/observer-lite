from datetime import datetime

from pydantic import BaseModel


class MaintenanceCreate(BaseModel):
    name: str
    starts_at: datetime
    ends_at: datetime
    repeat_cron: str | None = None
    suppress_alerts: bool = True
    monitor_ids: list[int] = []


class MaintenanceUpdate(BaseModel):
    name: str | None = None
    starts_at: datetime | None = None
    ends_at: datetime | None = None
    repeat_cron: str | None = None
    suppress_alerts: bool | None = None
    monitor_ids: list[int] | None = None


class MaintenanceOut(BaseModel):
    id: int
    name: str
    starts_at: datetime
    ends_at: datetime
    repeat_cron: str | None
    suppress_alerts: bool
    created_at: datetime
    monitor_ids: list[int] = []

    model_config = {"from_attributes": True}
