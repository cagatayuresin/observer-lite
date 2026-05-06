from sqlalchemy import Boolean, ForeignKey, Integer
from sqlalchemy.orm import Mapped, mapped_column, relationship

from app.db.session import Base


class MonitorUser(Base):
    __tablename__ = "monitor_users"

    monitor_id: Mapped[int] = mapped_column(Integer, ForeignKey("monitors.id", ondelete="CASCADE"), primary_key=True)
    user_id: Mapped[int] = mapped_column(Integer, ForeignKey("users.id", ondelete="CASCADE"), primary_key=True)
    notify: Mapped[bool] = mapped_column(Boolean, nullable=False, default=True)

    monitor: Mapped["Monitor"] = relationship("Monitor", back_populates="monitor_users")
    user: Mapped["User"] = relationship("User", back_populates="monitor_users")
