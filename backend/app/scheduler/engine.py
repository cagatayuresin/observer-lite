"""APScheduler engine configuration.

Creates a single :class:`~apscheduler.schedulers.asyncio.AsyncIOScheduler`
instance that is shared across the entire application.  Jobs are persisted
to the same SQLite database used by the application (in the
``apscheduler_jobs`` table), so they survive container restarts.

Key settings:
- ``coalesce=True`` — if a job fires multiple times while the scheduler is
  paused, only one execution is triggered on resume.
- ``max_instances=1`` — prevents a slow check from stacking up behind itself.
- ``misfire_grace_time=30`` — jobs that fire up to 30 s late are still run.
"""

from apscheduler.executors.asyncio import AsyncIOExecutor
from apscheduler.jobstores.sqlalchemy import SQLAlchemyJobStore
from apscheduler.schedulers.asyncio import AsyncIOScheduler

from app.config import get_settings

settings = get_settings()

_sync_db_url = settings.database_url.replace("sqlite+aiosqlite://", "sqlite://")

scheduler = AsyncIOScheduler(
    jobstores={
        "default": SQLAlchemyJobStore(url=_sync_db_url),
    },
    executors={
        "default": AsyncIOExecutor(),
    },
    job_defaults={
        "coalesce": True,
        "max_instances": 1,
        "misfire_grace_time": 30,
    },
)
