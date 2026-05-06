from pydantic import BaseModel


class MonitorStats(BaseModel):
    monitor_id: int
    uptime_percent: float
    avg_response_time_ms: float | None
    total_checks: int
    total_incidents: int
    open_incidents: int
    period_days: int
