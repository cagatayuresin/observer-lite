use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use crate::extract::AuthUser;
use crate::routes::users::stamp;
use crate::state::AppState;
use crate::timeutil::{format_db, now, parse_db};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/monitors/{monitor_id}/stats", get(stats))
        .route("/api/check-results", get(history))
        .route("/api/heartbeat/{token}", get(heartbeat).post(heartbeat))
        .route("/api/health", get(health))
        .route("/api/docs", get(docs))
}

#[derive(Deserialize)]
struct StatsQuery {
    #[serde(default = "days_default")]
    days: i64,
}

fn days_default() -> i64 {
    30
}

async fn stats(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(monitor_id): Path<i64>,
    Query(query): Query<StatsQuery>,
) -> AppResult<Json<Value>> {
    if !(1..=365).contains(&query.days) {
        return Err(AppError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "days must be between 1 and 365",
        ));
    }
    let since = format_db(now() - chrono::Duration::days(query.days));
    let counts: (i64, Option<i64>, Option<f64>) = sqlx::query_as(
        "SELECT COUNT(*),
                SUM(CASE WHEN status IN ('up', 'warning') THEN 1 ELSE 0 END),
                AVG(response_time_ms)
         FROM check_results
         WHERE monitor_id = ? AND checked_at >= ?",
    )
    .bind(monitor_id)
    .bind(&since)
    .fetch_one(&state.db)
    .await?;
    let total = counts.0;
    let up = counts.1.unwrap_or(0);
    let uptime = if total > 0 {
        round_places(up as f64 / total as f64 * 100.0, 3)
    } else {
        100.0
    };
    let avg = counts.2.map(|value| round_places(value, 1));
    let incidents: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM incidents WHERE monitor_id = ? AND started_at >= ?")
            .bind(monitor_id)
            .bind(&since)
            .fetch_one(&state.db)
            .await?;
    let open: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM incidents WHERE monitor_id = ? AND resolved_at IS NULL",
    )
    .bind(monitor_id)
    .fetch_one(&state.db)
    .await?;
    Ok(Json(json!({
        "monitor_id": monitor_id,
        "uptime_percent": uptime,
        "avg_response_time_ms": avg,
        "total_checks": total,
        "total_incidents": incidents.0,
        "open_incidents": open.0,
        "period_days": query.days,
    })))
}

#[derive(Deserialize)]
struct HistoryQuery {
    monitor_id: Option<i64>,
    from: Option<String>,
    to: Option<String>,
    #[serde(default = "history_limit")]
    limit: i64,
}

fn history_limit() -> i64 {
    200
}

async fn history(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<HistoryQuery>,
) -> AppResult<Json<Vec<Value>>> {
    if query.limit > 1000 {
        return Err(AppError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "limit must be at most 1000",
        ));
    }
    let mut sql = String::from(
        "SELECT id, monitor_id, checked_at, status, response_time_ms, status_code, is_ssl_valid, ssl_expiry_days, error_message
         FROM check_results WHERE 1 = 1",
    );
    let monitor_id = query.monitor_id.filter(|id| *id != 0);
    let from = query.from.as_deref().and_then(parse_db).map(format_db);
    let to = query.to.as_deref().and_then(parse_db).map(format_db);
    if monitor_id.is_some() {
        sql.push_str(" AND monitor_id = ?");
    }
    if from.is_some() {
        sql.push_str(" AND checked_at >= ?");
    }
    if to.is_some() {
        sql.push_str(" AND checked_at <= ?");
    }
    sql.push_str(" ORDER BY checked_at DESC LIMIT ?");
    let mut q = sqlx::query_as::<_, CheckRow>(sqlx::AssertSqlSafe(sql));
    if let Some(id) = monitor_id {
        q = q.bind(id);
    }
    if let Some(from) = from {
        q = q.bind(from);
    }
    if let Some(to) = to {
        q = q.bind(to);
    }
    let rows = q.bind(query.limit.max(1)).fetch_all(&state.db).await?;
    Ok(Json(rows.into_iter().map(check_json).collect()))
}

#[derive(sqlx::FromRow)]
struct CheckRow {
    id: i64,
    monitor_id: i64,
    checked_at: String,
    status: String,
    response_time_ms: Option<i64>,
    status_code: Option<i64>,
    is_ssl_valid: Option<i64>,
    ssl_expiry_days: Option<i64>,
    error_message: Option<String>,
}

fn check_json(row: CheckRow) -> Value {
    json!({
        "id": row.id,
        "monitor_id": row.monitor_id,
        "checked_at": stamp(&row.checked_at),
        "status": row.status,
        "response_time_ms": row.response_time_ms,
        "status_code": row.status_code,
        "is_ssl_valid": row.is_ssl_valid.map(|value| value != 0),
        "ssl_expiry_days": row.ssl_expiry_days,
        "error_message": row.error_message,
    })
}

async fn heartbeat(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> AppResult<Json<Value>> {
    let row: Option<(i64, String)> =
        sqlx::query_as("SELECT id, name FROM monitors WHERE heartbeat_token = ?")
            .bind(&token)
            .fetch_optional(&state.db)
            .await?;
    let Some((id, name)) = row else {
        return Err(AppError::new(
            StatusCode::NOT_FOUND,
            "Unknown heartbeat token",
        ));
    };
    let stamp_now = format_db(now());
    sqlx::query(
        "UPDATE monitors
         SET heartbeat_last_ping = ?, current_status = 'up', last_checked_at = ?,
             consecutive_failures = 0, updated_at = ?
         WHERE id = ?",
    )
    .bind(&stamp_now)
    .bind(&stamp_now)
    .bind(format_db(now()))
    .bind(id)
    .execute(&state.db)
    .await?;
    Ok(Json(json!({"message": "ok", "monitor": name})))
}

async fn health(State(state): State<AppState>) -> Json<Value> {
    Json(json!({"status": "ok", "version": state.settings.version}))
}

async fn docs(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "title": state.settings.app_name,
        "version": state.settings.version,
        "health": "/api/health",
    }))
}

fn round_places(value: f64, places: i32) -> f64 {
    let factor = 10f64.powi(places);
    (value * factor).round() / factor
}
