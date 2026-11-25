use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::audit;
use crate::error::{AppError, AppResult};
use crate::extract::{AdminUser, AuthUser, ClientIp};
use crate::routes::users::stamp;
use crate::state::AppState;
use crate::timeutil::{format_db, now, parse_db};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/maintenance", get(list).post(create))
        .route(
            "/api/maintenance/{window_id}",
            get(get_one).put(update).delete(remove),
        )
}

#[derive(Deserialize)]
struct MaintenanceBody {
    name: String,
    starts_at: String,
    ends_at: String,
    #[serde(default)]
    repeat_cron: Option<String>,
    #[serde(default = "yes")]
    suppress_alerts: bool,
    #[serde(default)]
    monitor_ids: Vec<i64>,
}

fn yes() -> bool {
    true
}

#[derive(Deserialize)]
struct MaintenancePatch {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    starts_at: Option<String>,
    #[serde(default)]
    ends_at: Option<String>,
    #[serde(default)]
    repeat_cron: Option<String>,
    #[serde(default)]
    suppress_alerts: Option<bool>,
    #[serde(default)]
    monitor_ids: Option<Vec<i64>>,
}

async fn list(State(state): State<AppState>, _user: AuthUser) -> AppResult<Json<Vec<Value>>> {
    let rows: Vec<(i64,)> =
        sqlx::query_as("SELECT id FROM maintenance_windows ORDER BY starts_at DESC")
            .fetch_all(&state.db)
            .await?;
    let mut out = Vec::new();
    for (id,) in rows {
        out.push(load(&state, id).await?);
    }
    Ok(Json(out))
}

async fn create(
    State(state): State<AppState>,
    user: AdminUser,
    ClientIp(ip): ClientIp,
    Json(body): Json<MaintenanceBody>,
) -> AppResult<(StatusCode, Json<Value>)> {
    let starts = store_time(&body.starts_at)?;
    let ends = store_time(&body.ends_at)?;
    let stamp_now = format_db(now());
    let id = sqlx::query(
        "INSERT INTO maintenance_windows (name, starts_at, ends_at, repeat_cron, suppress_alerts, created_by, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&body.name)
    .bind(&starts)
    .bind(&ends)
    .bind(&body.repeat_cron)
    .bind(i64::from(body.suppress_alerts))
    .bind(user.id)
    .bind(&stamp_now)
    .execute(&state.db)
    .await?
    .last_insert_rowid();
    replace_monitors(&state, id, &body.monitor_ids).await?;
    audit::write(
        &state.db,
        Some(user.id),
        "maintenance.create",
        Some("maintenance"),
        Some(id),
        Some(&body.name),
        ip.as_deref(),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(load(&state, id).await?)))
}

async fn get_one(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    Ok(Json(load(&state, id).await?))
}

async fn update(
    State(state): State<AppState>,
    user: AdminUser,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
    Json(body): Json<MaintenancePatch>,
) -> AppResult<Json<Value>> {
    let _ = load(&state, id).await?;
    if let Some(name) = &body.name {
        sqlx::query("UPDATE maintenance_windows SET name = ? WHERE id = ?")
            .bind(name)
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    if let Some(starts) = &body.starts_at {
        sqlx::query("UPDATE maintenance_windows SET starts_at = ? WHERE id = ?")
            .bind(store_time(starts)?)
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    if let Some(ends) = &body.ends_at {
        sqlx::query("UPDATE maintenance_windows SET ends_at = ? WHERE id = ?")
            .bind(store_time(ends)?)
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    if let Some(cron) = &body.repeat_cron {
        sqlx::query("UPDATE maintenance_windows SET repeat_cron = ? WHERE id = ?")
            .bind(cron)
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    if let Some(flag) = body.suppress_alerts {
        sqlx::query("UPDATE maintenance_windows SET suppress_alerts = ? WHERE id = ?")
            .bind(i64::from(flag))
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    if let Some(ids) = &body.monitor_ids {
        replace_monitors(&state, id, ids).await?;
    }
    audit::write(
        &state.db,
        Some(user.id),
        "maintenance.update",
        Some("maintenance"),
        Some(id),
        None,
        ip.as_deref(),
    )
    .await?;
    Ok(Json(load(&state, id).await?))
}

async fn remove(
    State(state): State<AppState>,
    user: AdminUser,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    let changed = sqlx::query("DELETE FROM maintenance_windows WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?
        .rows_affected();
    if changed == 0 {
        return Err(AppError::new(
            StatusCode::NOT_FOUND,
            "Maintenance window not found",
        ));
    }
    audit::write(
        &state.db,
        Some(user.id),
        "maintenance.delete",
        Some("maintenance"),
        Some(id),
        None,
        ip.as_deref(),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn replace_monitors(state: &AppState, window_id: i64, ids: &[i64]) -> AppResult<()> {
    sqlx::query("DELETE FROM maintenance_window_monitors WHERE window_id = ?")
        .bind(window_id)
        .execute(&state.db)
        .await?;
    for monitor_id in ids {
        sqlx::query(
            "INSERT INTO maintenance_window_monitors (window_id, monitor_id) VALUES (?, ?)",
        )
        .bind(window_id)
        .bind(monitor_id)
        .execute(&state.db)
        .await?;
    }
    Ok(())
}

type WindowRow = (i64, String, String, String, Option<String>, i64, String);

async fn load(state: &AppState, id: i64) -> AppResult<Value> {
    let row: Option<WindowRow> = sqlx::query_as(
        "SELECT id, name, starts_at, ends_at, repeat_cron, suppress_alerts, created_at FROM maintenance_windows WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?;
    let Some((id, name, starts, ends, cron, suppress, created)) = row else {
        return Err(AppError::new(
            StatusCode::NOT_FOUND,
            "Maintenance window not found",
        ));
    };
    let monitors: Vec<(i64,)> =
        sqlx::query_as("SELECT monitor_id FROM maintenance_window_monitors WHERE window_id = ? ORDER BY monitor_id")
            .bind(id)
            .fetch_all(&state.db)
            .await?;
    Ok(json!({
        "id": id,
        "name": name,
        "starts_at": stamp(&starts),
        "ends_at": stamp(&ends),
        "repeat_cron": cron,
        "suppress_alerts": suppress != 0,
        "created_at": stamp(&created),
        "monitor_ids": monitors.into_iter().map(|(id,)| id).collect::<Vec<_>>(),
    }))
}

fn store_time(value: &str) -> AppResult<String> {
    parse_db(value)
        .map(format_db)
        .ok_or_else(|| AppError::new(StatusCode::UNPROCESSABLE_ENTITY, "Invalid datetime"))
}
