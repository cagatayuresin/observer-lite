use serde_json::json;
use sqlx::SqlitePool;

use crate::checkers::http::{http_check, http_method, HttpProbe};
use crate::checkers::ping::ping_check;
use crate::checkers::ssl::ssl_check;
use crate::checkers::CheckOutcome;
use crate::db::flag;
use crate::error::AppError;
use crate::services::notify::{notify_down, notify_recovery, notify_ssl_warning};
use crate::state::AppState;
use crate::timeutil::{format_api, format_db, now, parse_db};

#[derive(Debug, sqlx::FromRow)]
struct MonitorRow {
    id: i64,
    name: String,
    url: String,
    monitor_type: String,
    is_enabled: i64,
    check_interval_seconds: i64,
    timeout_seconds: i64,
    retry_count: i64,
    request_headers: Option<String>,
    request_body: Option<String>,
    expected_status_codes: String,
    expected_body_type: Option<String>,
    expected_body_value: Option<String>,
    response_time_warning_ms: i64,
    ssl_check_enabled: i64,
    ssl_expiry_warning_days: i64,
    alerts_enabled: i64,
    heartbeat_grace_seconds: i64,
    heartbeat_last_ping: Option<String>,
    current_status: String,
    consecutive_failures: i64,
}

pub async fn run_monitor_check(state: &AppState, monitor_id: i64) {
    let permit = match state.checks.acquire().await {
        Ok(permit) => permit,
        Err(_) => return,
    };
    if let Err(err) = check_inner(state, monitor_id).await {
        tracing::error!(monitor_id, error = %err.detail, "monitor check failed");
    }
    drop(permit);
}

async fn check_inner(state: &AppState, monitor_id: i64) -> Result<(), AppError> {
    let Some(monitor) = load_monitor(&state.db, monitor_id).await? else {
        return Ok(());
    };
    if !flag(monitor.is_enabled) || in_maintenance(&state.db, monitor_id).await? {
        return Ok(());
    }
    let mut outcome = if monitor.monitor_type == "heartbeat" {
        heartbeat_outcome(&monitor)
    } else if monitor.monitor_type == "ping" {
        ping_check(
            &monitor.url,
            monitor.timeout_seconds,
            monitor.response_time_warning_ms,
        )
        .await
    } else {
        http_check(
            &state.http,
            HttpProbe {
                method: http_method(&monitor.monitor_type),
                url: &monitor.url,
                headers_json: monitor.request_headers.as_deref(),
                body: monitor.request_body.as_deref(),
                expected_status: &monitor.expected_status_codes,
                body_type: monitor.expected_body_type.as_deref(),
                body_value: monitor.expected_body_value.as_deref(),
                timeout_seconds: monitor.timeout_seconds,
                warning_ms: monitor.response_time_warning_ms,
            },
        )
        .await
    };

    if flag(monitor.ssl_check_enabled) && monitor.url.starts_with("https://") {
        let info = ssl_check(&monitor.url).await;
        outcome.is_ssl_valid = Some(info.is_valid);
        outcome.ssl_expiry_days = info.days;
        if let Some(days) = info.days {
            if days <= monitor.ssl_expiry_warning_days {
                notify_ssl_warning(state, monitor.id, &monitor.name, &monitor.url, days).await;
            }
        }
    }
    process_result(state, &monitor, outcome).await
}

fn heartbeat_outcome(monitor: &MonitorRow) -> CheckOutcome {
    let last = monitor.heartbeat_last_ping.as_deref().and_then(parse_db);
    let limit = monitor.check_interval_seconds + monitor.heartbeat_grace_seconds;
    let fresh = last.is_some_and(|ping| (now() - ping).num_seconds() <= limit);
    if fresh {
        CheckOutcome {
            status: "up".into(),
            ..CheckOutcome::default()
        }
    } else {
        CheckOutcome::down(0, "Heartbeat not received in time")
    }
}

async fn process_result(
    state: &AppState,
    monitor: &MonitorRow,
    result: CheckOutcome,
) -> Result<(), AppError> {
    let checked = result.checked_at;
    let checked_db = format_db(checked);
    let is_failure = result.status == "down";
    let failures = if is_failure {
        monitor.consecutive_failures + 1
    } else {
        0
    };
    let prev_status = monitor.current_status.clone();

    let mut tx = state.db.begin().await?;
    sqlx::query(
        "INSERT INTO check_results
         (monitor_id, checked_at, status, response_time_ms, status_code, is_ssl_valid, ssl_expiry_days, error_message)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(monitor.id)
    .bind(&checked_db)
    .bind(&result.status)
    .bind(result.response_time_ms)
    .bind(result.status_code)
    .bind(result.is_ssl_valid.map(i64::from))
    .bind(result.ssl_expiry_days)
    .bind(&result.error_message)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE monitors
         SET current_status = ?, last_checked_at = ?, last_response_time_ms = ?,
             consecutive_failures = ?, updated_at = ?
         WHERE id = ?",
    )
    .bind(&result.status)
    .bind(&checked_db)
    .bind(result.response_time_ms)
    .bind(failures)
    .bind(format_db(now()))
    .bind(monitor.id)
    .execute(&mut *tx)
    .await?;

    let mut notify_down_now = false;
    let mut notify_recovery_now = false;
    let mut started_at = String::new();
    let mut resolved_at = String::new();
    let mut duration = None;

    if is_failure {
        if failures >= monitor.retry_count.max(1) {
            let open: Option<(i64, i64, String)> = sqlx::query_as(
                "SELECT id, notification_sent, started_at FROM incidents
                 WHERE monitor_id = ? AND resolved_at IS NULL ORDER BY id DESC LIMIT 1",
            )
            .bind(monitor.id)
            .fetch_optional(&mut *tx)
            .await?;
            let (incident_id, notification_sent, started) = if let Some(row) = open {
                row
            } else {
                let started = format_db(now());
                let result_id = sqlx::query(
                    "INSERT INTO incidents (monitor_id, started_at, root_cause, notification_sent, recovery_sent)
                     VALUES (?, ?, ?, 0, 0)",
                )
                .bind(monitor.id)
                .bind(&started)
                .bind(&result.error_message)
                .execute(&mut *tx)
                .await?
                .last_insert_rowid();
                (result_id, 0, started)
            };
            started_at = display_time(&started);
            if flag(monitor.alerts_enabled) && notification_sent == 0 {
                sqlx::query("UPDATE incidents SET notification_sent = 1 WHERE id = ?")
                    .bind(incident_id)
                    .execute(&mut *tx)
                    .await?;
                notify_down_now = true;
            }
        }
    } else if let Some((incident_id, recovery_sent, started)) =
        sqlx::query_as::<_, (i64, i64, String)>(
            "SELECT id, recovery_sent, started_at FROM incidents
         WHERE monitor_id = ? AND resolved_at IS NULL ORDER BY id DESC LIMIT 1",
        )
        .bind(monitor.id)
        .fetch_optional(&mut *tx)
        .await?
    {
        let resolved = now();
        resolved_at = format_db(resolved);
        let started_dt = parse_db(&started).unwrap_or(resolved);
        duration = Some((resolved - started_dt).num_seconds().max(0));
        sqlx::query("UPDATE incidents SET resolved_at = ?, duration_seconds = ? WHERE id = ?")
            .bind(&resolved_at)
            .bind(duration)
            .bind(incident_id)
            .execute(&mut *tx)
            .await?;
        if flag(monitor.alerts_enabled) && recovery_sent == 0 {
            sqlx::query("UPDATE incidents SET recovery_sent = 1 WHERE id = ?")
                .bind(incident_id)
                .execute(&mut *tx)
                .await?;
            notify_recovery_now = true;
        }
    }
    tx.commit().await?;

    if notify_down_now {
        notify_down(
            state,
            monitor.id,
            &monitor.name,
            &monitor.url,
            &started_at,
            result.error_message.as_deref(),
        )
        .await;
    }
    if notify_recovery_now {
        notify_recovery(
            state,
            monitor.id,
            &monitor.name,
            &monitor.url,
            &display_time(&resolved_at),
            duration,
        )
        .await;
    }

    state.sse.publish(
        "monitor.check_result",
        &json!({
            "monitor_id": monitor.id,
            "status": result.status,
            "prev_status": prev_status,
            "response_time_ms": result.response_time_ms,
            "checked_at": format_api(checked),
        }),
    );
    Ok(())
}

fn display_time(stored: &str) -> String {
    parse_db(stored)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| stored.to_string())
}

async fn load_monitor(pool: &SqlitePool, id: i64) -> Result<Option<MonitorRow>, AppError> {
    Ok(sqlx::query_as::<_, MonitorRow>(
        "SELECT id, name, url, monitor_type, is_enabled, check_interval_seconds, timeout_seconds,
                retry_count, request_headers, request_body, expected_status_codes, expected_body_type,
                expected_body_value, response_time_warning_ms, ssl_check_enabled, ssl_expiry_warning_days,
                alerts_enabled, heartbeat_grace_seconds, heartbeat_last_ping, current_status, consecutive_failures
         FROM monitors WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?)
}

pub async fn in_maintenance(pool: &SqlitePool, monitor_id: i64) -> Result<bool, AppError> {
    let stamp = format_db(now());
    let found: Option<(i64,)> = sqlx::query_as(
        "SELECT w.id FROM maintenance_windows w
         JOIN maintenance_window_monitors m ON m.window_id = w.id
         WHERE m.monitor_id = ? AND w.starts_at <= ? AND w.ends_at >= ?
         LIMIT 1",
    )
    .bind(monitor_id)
    .bind(&stamp)
    .bind(&stamp)
    .fetch_optional(pool)
    .await?;
    Ok(found.is_some())
}

pub async fn run_daily_ssl_scan(state: &AppState) {
    let rows: Vec<(i64, String, String, i64)> = match sqlx::query_as(
        "SELECT id, name, url, ssl_expiry_warning_days FROM monitors
         WHERE is_enabled = 1 AND ssl_check_enabled = 1
           AND monitor_type IN ('http_get', 'http_post', 'http_head')
           AND url LIKE 'https://%'",
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(rows) => rows,
        Err(err) => {
            tracing::error!(error = %err, "ssl scan query failed");
            return;
        }
    };
    for (id, name, url, warning_days) in rows {
        let info = ssl_check(&url).await;
        if let Some(days) = info.days {
            if days <= warning_days {
                notify_ssl_warning(state, id, &name, &url, days).await;
            }
        }
    }
}
