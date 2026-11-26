use chrono::Timelike;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::Notify;

use crate::services::checks::run_monitor_check;
use crate::services::retention::run_retention;
use crate::state::AppState;

struct Job {
    stop: Arc<AtomicBool>,
    notify: Arc<Notify>,
}

/// One in-process loop per monitor. The `monitors` table is the only schedule.
pub struct Scheduler {
    jobs: Mutex<HashMap<i64, Job>>,
    daily: Mutex<Option<Job>>,
    enabled: bool,
}

impl Scheduler {
    pub fn new(enabled: bool) -> Self {
        Self {
            jobs: Mutex::new(HashMap::new()),
            daily: Mutex::new(None),
            enabled,
        }
    }

    pub fn spawn_daily(&self, state: AppState) {
        if !self.enabled {
            return;
        }
        let job = Job::new();
        let stop = Arc::clone(&job.stop);
        let notify = Arc::clone(&job.notify);
        *self.daily.lock().expect("daily job") = Some(job);
        tokio::spawn(daily_loop(state, stop, notify));
    }

    pub fn upsert(&self, state: AppState, id: i64, interval_seconds: i64) {
        if !self.enabled {
            return;
        }
        self.remove(id);
        let job = Job::new();
        let stop = Arc::clone(&job.stop);
        let notify = Arc::clone(&job.notify);
        self.jobs.lock().expect("scheduler").insert(id, job);
        let interval = Duration::from_secs(interval_seconds.max(1) as u64);
        tokio::spawn(monitor_loop(state, id, interval, stop, notify));
    }

    pub fn pause(&self, id: i64) {
        self.remove(id);
    }

    pub fn remove(&self, id: i64) {
        if let Some(job) = self.jobs.lock().expect("scheduler").remove(&id) {
            job.cancel();
        }
    }

    pub fn shutdown(&self) {
        let mut jobs = self.jobs.lock().expect("scheduler");
        for (_, job) in jobs.drain() {
            job.cancel();
        }
        if let Some(job) = self.daily.lock().expect("daily job").take() {
            job.cancel();
        }
    }
}

impl Job {
    fn new() -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(false)),
            notify: Arc::new(Notify::new()),
        }
    }

    fn cancel(&self) {
        self.stop.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }
}

async fn monitor_loop(
    state: AppState,
    id: i64,
    interval: Duration,
    stop: Arc<AtomicBool>,
    notify: Arc<Notify>,
) {
    loop {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        run_monitor_check(&state, id).await;
        if stop.load(Ordering::SeqCst) {
            break;
        }
        tokio::select! {
            _ = notify.notified() => break,
            _ = tokio::time::sleep(interval) => {}
        }
    }
}

async fn daily_loop(state: AppState, stop: Arc<AtomicBool>, notify: Arc<Notify>) {
    loop {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        let wait = seconds_until_next(&[2, 6]);
        tokio::select! {
            _ = notify.notified() => break,
            _ = tokio::time::sleep(Duration::from_secs(wait)) => {}
        }
        if stop.load(Ordering::SeqCst) {
            break;
        }
        run_scheduled_hour(&state, chrono::Local::now().hour()).await;
    }
}

async fn run_scheduled_hour(state: &AppState, hour: u32) {
    if hour == 2 {
        if let Err(err) = run_retention(state).await {
            tracing::error!(error = %err.detail, "retention cleanup failed");
        }
    } else if hour == 6 {
        crate::services::checks::run_daily_ssl_scan(state).await;
    }
}

fn seconds_until_next(hours: &[u32]) -> u64 {
    let now = chrono::Local::now();
    let mut best: Option<i64> = None;
    for hour in hours {
        let candidate = now.date_naive().and_hms_opt(*hour, 0, 0);
        let candidate =
            candidate.and_then(|naive| naive.and_local_timezone(chrono::Local).single());
        let candidate = match candidate {
            Some(when) if when > now => Some(when),
            _ => (now.date_naive() + chrono::Duration::days(1))
                .and_hms_opt(*hour, 0, 0)
                .and_then(|naive| naive.and_local_timezone(chrono::Local).single()),
        };
        if let Some(when) = candidate {
            let secs = (when - now).num_seconds().max(1);
            best = Some(best.map(|current| current.min(secs)).unwrap_or(secs));
        }
    }
    best.unwrap_or(3600) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Settings;
    use crate::state::bootstrap;

    #[test]
    fn next_daily_slot_is_within_a_day() {
        let wait = seconds_until_next(&[2, 6]);
        assert!((1..=86_400).contains(&wait));
    }

    #[tokio::test]
    async fn hour_jobs_and_loops_shut_down() {
        let dir = tempfile::tempdir().unwrap();
        let mut settings =
            Settings::for_test(dir.path().join("observer.db").to_string_lossy().to_string());
        settings.scheduler_enabled = true;
        let state = bootstrap(settings).await.unwrap();
        state.scheduler.upsert(state.clone(), 999_999, 3600);
        run_scheduled_hour(&state, 2).await;
        run_scheduled_hour(&state, 6).await;
        run_scheduled_hour(&state, 12).await;
        tokio::time::sleep(Duration::from_millis(30)).await;
        state.scheduler.shutdown();
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
}
