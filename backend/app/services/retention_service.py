"""Data retention: purge old check_results rows on a nightly schedule.

Deletion is done in batches of :data:`_BATCH_SIZE` rows to avoid creating a
single massive transaction that would stall reads on the SQLite WAL file.
The retention period is read from ``app_settings`` at job runtime, so
administrators can change it without restarting the application.
"""

import logging
from datetime import datetime, timedelta, timezone

from sqlalchemy import delete, select

from app.db.models import AppSetting, CheckResult
from app.db.session import AsyncSessionLocal

logger = logging.getLogger(__name__)

_BATCH_SIZE = 10_000


async def run_retention_cleanup() -> None:
    async with AsyncSessionLocal() as db:
        # Get configured retention days
        result = await db.execute(
            select(AppSetting).where(AppSetting.key == "data_retention_days")
        )
        setting = result.scalar_one_or_none()
        retention_days = int(setting.value) if setting else 90

        cutoff = datetime.now(timezone.utc) - timedelta(days=retention_days)
        total_deleted = 0

        while True:
            # Batch delete to avoid one massive transaction
            subq = (
                select(CheckResult.id)
                .where(CheckResult.checked_at < cutoff)
                .limit(_BATCH_SIZE)
            )
            result = await db.execute(subq)
            ids = [row[0] for row in result.all()]
            if not ids:
                break
            await db.execute(delete(CheckResult).where(CheckResult.id.in_(ids)))
            await db.commit()
            total_deleted += len(ids)

        if total_deleted:
            logger.info("Retention cleanup: deleted %d check_results older than %d days", total_deleted, retention_days)
