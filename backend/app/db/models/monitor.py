from datetime import datetime

from sqlalchemy import Boolean, DateTime, ForeignKey, Integer, String, Text
from sqlalchemy.orm import Mapped, mapped_column, relationship

from app.db.session import Base


class Monitor(Base):
    __tablename__ = "monitors"

    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    name: Mapped[str] = mapped_column(String(128), nullable=False)
    url: Mapped[str] = mapped_column(Text, nullable=False)
    monitor_type: Mapped[str] = mapped_column(String(16), nullable=False, default="http_get")
    group_id: Mapped[int | None] = mapped_column(Integer, ForeignKey("monitor_groups.id", ondelete="SET NULL"), nullable=True, index=True)

    is_enabled: Mapped[bool] = mapped_column(Boolean, nullable=False, default=True)
    check_interval_seconds: Mapped[int] = mapped_column(Integer, nullable=False, default=60)
    timeout_seconds: Mapped[int] = mapped_column(Integer, nullable=False, default=10)
    retry_count: Mapped[int] = mapped_column(Integer, nullable=False, default=3)
    retry_interval_seconds: Mapped[int] = mapped_column(Integer, nullable=False, default=20)
    alert_cooldown_seconds: Mapped[int] = mapped_column(Integer, nullable=False, default=1800)

    # HTTP-specific
    request_headers: Mapped[str | None] = mapped_column(Text, nullable=True)   # JSON
    request_body: Mapped[str | None] = mapped_column(Text, nullable=True)
    expected_status_codes: Mapped[str] = mapped_column(String(128), nullable=False, default="2xx")
    expected_body_type: Mapped[str | None] = mapped_column(String(16), nullable=True)  # contains|equals|not_equals
    expected_body_value: Mapped[str | None] = mapped_column(Text, nullable=True)
    response_time_warning_ms: Mapped[int] = mapped_column(Integer, nullable=False, default=2000)

    # SSL
    ssl_check_enabled: Mapped[bool] = mapped_column(Boolean, nullable=False, default=True)
    ssl_expiry_warning_days: Mapped[int] = mapped_column(Integer, nullable=False, default=30)

    # Alerts
    alerts_enabled: Mapped[bool] = mapped_column(Boolean, nullable=False, default=True)

    # Heartbeat
    heartbeat_token: Mapped[str | None] = mapped_column(String(64), unique=True, nullable=True, index=True)
    heartbeat_grace_seconds: Mapped[int] = mapped_column(Integer, nullable=False, default=60)
    heartbeat_last_ping: Mapped[datetime | None] = mapped_column(DateTime(timezone=True), nullable=True)

    # Denormalized live state
    current_status: Mapped[str] = mapped_column(String(16), nullable=False, default="pending", index=True)
    last_checked_at: Mapped[datetime | None] = mapped_column(DateTime(timezone=True), nullable=True)
    last_response_time_ms: Mapped[int | None] = mapped_column(Integer, nullable=True)
    consecutive_failures: Mapped[int] = mapped_column(Integer, nullable=False, default=0)

    created_by: Mapped[int | None] = mapped_column(Integer, ForeignKey("users.id", ondelete="SET NULL"), nullable=True)
    created_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), nullable=False)
    updated_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), nullable=False)

    group: Mapped["MonitorGroup | None"] = relationship("MonitorGroup", back_populates="monitors")
    monitor_users: Mapped[list["MonitorUser"]] = relationship("MonitorUser", back_populates="monitor", cascade="all, delete-orphan")
    check_results: Mapped[list["CheckResult"]] = relationship("CheckResult", back_populates="monitor", cascade="all, delete-orphan")
    incidents: Mapped[list["Incident"]] = relationship("Incident", back_populates="monitor", cascade="all, delete-orphan")
    notification_channels: Mapped[list["MonitorNotificationChannel"]] = relationship("MonitorNotificationChannel", back_populates="monitor", cascade="all, delete-orphan")
    maintenance_monitors: Mapped[list["MaintenanceWindowMonitor"]] = relationship("MaintenanceWindowMonitor", back_populates="monitor", cascade="all, delete-orphan")
