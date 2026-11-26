use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::audit;
use crate::error::{AppError, AppResult};
use crate::extract::{AdminUser, AuthUser, ClientIp};
use crate::routes::users::stamp;
use crate::state::AppState;
use crate::timeutil::{format_db, now};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/incidents", get(list))
        .route("/api/incidents/{incident_id}", get(get_one))
        .route(
            "/api/incidents/{incident_id}/acknowledge",
            post(acknowledge),
        )
}

#[derive(Deserialize)]
struct IncidentQuery {
    monitor_id: Option<i64>,
    #[serde(default)]
    open_only: bool,
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_limit() -> i64 {
    100
}

async fn list(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<IncidentQuery>,
) -> AppResult<Json<Vec<Value>>> {
    if query.limit > 500 {
        return Err(AppError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "limit must be at most 500",
        ));
    }
    let mut sql = String::from(
        "SELECT id, monitor_id, started_at, resolved_at, duration_seconds, root_cause, notification_sent, recovery_sent, acknowledged_by, acknowledged_at
         FROM incidents WHERE 1 = 1",
    );
    if query.monitor_id.is_some_and(|id| id != 0) {
        sql.push_str(" AND monitor_id = ?");
    }
    if query.open_only {
        sql.push_str(" AND resolved_at IS NULL");
    }
    sql.push_str(" ORDER BY started_at DESC LIMIT ?");
    let mut q = sqlx::query_as::<_, IncidentRow>(sqlx::AssertSqlSafe(sql));
    if let Some(id) = query.monitor_id.filter(|id| *id != 0) {
        q = q.bind(id);
    }
    let rows = q.bind(query.limit.max(1)).fetch_all(&state.db).await?;
    Ok(Json(rows.into_iter().map(incident_json).collect()))
}

async fn get_one(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    Ok(Json(incident_json(fetch(&state, id).await?)))
}

async fn acknowledge(
    State(state): State<AppState>,
    user: AdminUser,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    let _ = fetch(&state, id).await?;
    let stamp_now = format_db(now());
    sqlx::query("UPDATE incidents SET acknowledged_by = ?, acknowledged_at = ? WHERE id = ?")
        .bind(user.id)
        .bind(&stamp_now)
        .bind(id)
        .execute(&state.db)
        .await?;
    audit::write(
        &state.db,
        Some(user.id),
        "incident.acknowledge",
        Some("incident"),
        Some(id),
        None,
        ip.as_deref(),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(sqlx::FromRow)]
struct IncidentRow {
    id: i64,
    monitor_id: i64,
    started_at: String,
    resolved_at: Option<String>,
    duration_seconds: Option<i64>,
    root_cause: Option<String>,
    notification_sent: i64,
    recovery_sent: i64,
    acknowledged_by: Option<i64>,
    acknowledged_at: Option<String>,
}

async fn fetch(state: &AppState, id: i64) -> AppResult<IncidentRow> {
    sqlx::query_as(
        "SELECT id, monitor_id, started_at, resolved_at, duration_seconds, root_cause, notification_sent, recovery_sent, acknowledged_by, acknowledged_at
         FROM incidents WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::new(StatusCode::NOT_FOUND, "Incident not found"))
}

fn incident_json(row: IncidentRow) -> Value {
    json!({
        "id": row.id,
        "monitor_id": row.monitor_id,
        "started_at": stamp(&row.started_at),
        "resolved_at": row.resolved_at.as_deref().map(stamp),
        "duration_seconds": row.duration_seconds,
        "root_cause": row.root_cause,
        "notification_sent": row.notification_sent != 0,
        "recovery_sent": row.recovery_sent != 0,
        "acknowledged_by": row.acknowledged_by,
        "acknowledged_at": row.acknowledged_at.as_deref().map(stamp),
    })
}
