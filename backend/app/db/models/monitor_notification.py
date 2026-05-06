from sqlalchemy import Boolean, ForeignKey, Integer
from sqlalchemy.orm import Mapped, mapped_column, relationship

from app.db.session import Base


class MonitorNotificationChannel(Base):
    __tablename__ = "monitor_notification_channels"

    monitor_id: Mapped[int] = mapped_column(Integer, ForeignKey("monitors.id", ondelete="CASCADE"), primary_key=True)
    channel_id: Mapped[int] = mapped_column(Integer, ForeignKey("notification_channels.id", ondelete="CASCADE"), primary_key=True)
    on_down: Mapped[bool] = mapped_column(Boolean, nullable=False, default=True)
    on_recovery: Mapped[bool] = mapped_column(Boolean, nullable=False, default=True)
    on_ssl_warn: Mapped[bool] = mapped_column(Boolean, nullable=False, default=True)

    monitor: Mapped["Monitor"] = relationship("Monitor", back_populates="notification_channels")
    channel: Mapped["NotificationChannel"] = relationship("NotificationChannel", back_populates="monitor_channels")
