from datetime import datetime

from pydantic import BaseModel


class ChannelCreate(BaseModel):
    name: str
    channel_type: str  # email|telegram
    config: dict
    is_enabled: bool = True


class ChannelUpdate(BaseModel):
    name: str | None = None
    config: dict | None = None
    is_enabled: bool | None = None


class ChannelOut(BaseModel):
    id: int
    name: str
    channel_type: str
    is_enabled: bool
    created_at: datetime

    model_config = {"from_attributes": True}


class MonitorChannelAssign(BaseModel):
    channel_id: int
    on_down: bool = True
    on_recovery: bool = True
    on_ssl_warn: bool = True
