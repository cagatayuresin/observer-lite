"""Datetime helpers for values read back from SQLite.

SQLite stores datetimes as strings without an offset, so SQLAlchemy returns
them naive even when the column is ``DateTime(timezone=True)``. Comparisons
with aware UTC timestamps need one conversion.
"""

from datetime import UTC, datetime


def ensure_utc(value: datetime) -> datetime:
    """Return *value* as an aware UTC datetime.

    Naive values are treated as UTC, which is how this app writes them.
    """
    if value.tzinfo is None:
        return value.replace(tzinfo=UTC)
    return value.astimezone(UTC)
