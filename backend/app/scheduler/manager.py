"""Monitor job lifecycle management for APScheduler.

Provides helpers that the API layer calls whenever a monitor is created,
updated, paused, resumed, or deleted — keeping the scheduler in sync with
the database without requiring a restart.
"""

from apscheduler.triggers.interval import IntervalTrigger

from app.db.models import Monitor
from app.scheduler.engine import scheduler


def _job_id(monitor_id: int) -> str:
    return f"monitor_{monitor_id}"


def upsert_monitor_job(monitor: Monitor) -> None:
    """Add or reschedule the APScheduler job for *monitor*.

    If a job with the same ID already exists, its trigger is updated
    in-place; otherwise a new job is registered.  The job calls
    :func:`~app.scheduler.jobs.run_monitor_check` with the monitor's ID.

    Args:
        monitor: The :class:`~app.db.models.Monitor` to schedule.
    """
    from app.scheduler.jobs import run_monitor_check

    job_id = _job_id(monitor.id)
    trigger = IntervalTrigger(seconds=monitor.check_interval_seconds)

    existing = scheduler.get_job(job_id)
    if existing:
        existing.reschedule(trigger=trigger)
    else:
        scheduler.add_job(
            run_monitor_check,
            trigger=trigger,
            id=job_id,
            args=[monitor.id],
            replace_existing=True,
        )


def remove_monitor_job(monitor_id: int) -> None:
    """Remove the scheduled job for *monitor_id*, if it exists."""
    job_id = _job_id(monitor_id)
    if scheduler.get_job(job_id):
        scheduler.remove_job(job_id)


def pause_monitor_job(monitor_id: int) -> None:
    """Pause the scheduled job for *monitor_id* without removing it."""
    job_id = _job_id(monitor_id)
    job = scheduler.get_job(job_id)
    if job:
        job.pause()


def resume_monitor_job(monitor_id: int) -> None:
    """Resume a previously paused job for *monitor_id*."""
    job_id = _job_id(monitor_id)
    job = scheduler.get_job(job_id)
    if job:
        job.resume()
