from datetime import datetime

from sqlalchemy import Boolean, DateTime, ForeignKey, Integer, String
from sqlalchemy.orm import Mapped, mapped_column, relationship

from app.db.session import Base


class MaintenanceWindow(Base):
    __tablename__ = "maintenance_windows"

    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    name: Mapped[str] = mapped_column(String(128), nullable=False)
    starts_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), nullable=False)
    ends_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), nullable=False)
    repeat_cron: Mapped[str | None] = mapped_column(String(64), nullable=True)
    suppress_alerts: Mapped[bool] = mapped_column(Boolean, nullable=False, default=True)
    created_by: Mapped[int | None] = mapped_column(Integer, ForeignKey("users.id", ondelete="SET NULL"), nullable=True)
    created_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), nullable=False)

    monitor_windows: Mapped[list["MaintenanceWindowMonitor"]] = relationship(
        "MaintenanceWindowMonitor", back_populates="window", cascade="all, delete-orphan"
    )


class MaintenanceWindowMonitor(Base):
    __tablename__ = "maintenance_window_monitors"

    window_id: Mapped[int] = mapped_column(Integer, ForeignKey("maintenance_windows.id", ondelete="CASCADE"), primary_key=True)
    monitor_id: Mapped[int] = mapped_column(Integer, ForeignKey("monitors.id", ondelete="CASCADE"), primary_key=True)

    window: Mapped["MaintenanceWindow"] = relationship("MaintenanceWindow", back_populates="monitor_windows")
    monitor: Mapped["Monitor"] = relationship("Monitor", back_populates="maintenance_monitors")
