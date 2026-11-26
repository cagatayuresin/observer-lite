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
use crate::timeutil::{format_db, now};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/groups", get(list).post(create))
        .route(
            "/api/groups/{group_id}",
            axum::routing::put(update).delete(remove),
        )
}

#[derive(Deserialize)]
struct GroupBody {
    name: String,
    #[serde(default)]
    description: Option<String>,
}

async fn list(State(state): State<AppState>, _user: AuthUser) -> AppResult<Json<Vec<Value>>> {
    let rows: Vec<(i64, String, Option<String>, String)> = sqlx::query_as(
        "SELECT id, name, description, created_at FROM monitor_groups ORDER BY name",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows.into_iter().map(group_json).collect()))
}

async fn create(
    State(state): State<AppState>,
    user: AdminUser,
    ClientIp(ip): ClientIp,
    Json(body): Json<GroupBody>,
) -> AppResult<(StatusCode, Json<Value>)> {
    if body.name.trim().is_empty() {
        return Err(AppError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Name is required",
        ));
    }
    let stamp_now = format_db(now());
    let id = sqlx::query(
        "INSERT INTO monitor_groups (name, description, created_by, created_at) VALUES (?, ?, ?, ?)",
    )
    .bind(body.name.trim())
    .bind(&body.description)
    .bind(user.id)
    .bind(&stamp_now)
    .execute(&state.db)
    .await?
    .last_insert_rowid();
    audit::write(
        &state.db,
        Some(user.id),
        "group.create",
        Some("group"),
        Some(id),
        Some(body.name.trim()),
        ip.as_deref(),
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(group_json((id, body.name, body.description, stamp_now))),
    ))
}

async fn update(
    State(state): State<AppState>,
    user: AdminUser,
    ClientIp(ip): ClientIp,
    Path(group_id): Path<i64>,
    Json(body): Json<GroupBody>,
) -> AppResult<Json<Value>> {
    let existing: Option<(String,)> =
        sqlx::query_as("SELECT created_at FROM monitor_groups WHERE id = ?")
            .bind(group_id)
            .fetch_optional(&state.db)
            .await?;
    let Some((created_at,)) = existing else {
        return Err(AppError::new(StatusCode::NOT_FOUND, "Group not found"));
    };
    sqlx::query("UPDATE monitor_groups SET name = ?, description = ? WHERE id = ?")
        .bind(&body.name)
        .bind(&body.description)
        .bind(group_id)
        .execute(&state.db)
        .await?;
    audit::write(
        &state.db,
        Some(user.id),
        "group.update",
        Some("group"),
        Some(group_id),
        None,
        ip.as_deref(),
    )
    .await?;
    Ok(Json(group_json((
        group_id,
        body.name,
        body.description,
        created_at,
    ))))
}

async fn remove(
    State(state): State<AppState>,
    user: AdminUser,
    ClientIp(ip): ClientIp,
    Path(group_id): Path<i64>,
) -> AppResult<StatusCode> {
    let changed = sqlx::query("DELETE FROM monitor_groups WHERE id = ?")
        .bind(group_id)
        .execute(&state.db)
        .await?
        .rows_affected();
    if changed == 0 {
        return Err(AppError::new(StatusCode::NOT_FOUND, "Group not found"));
    }
    audit::write(
        &state.db,
        Some(user.id),
        "group.delete",
        Some("group"),
        Some(group_id),
        None,
        ip.as_deref(),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

fn group_json(row: (i64, String, Option<String>, String)) -> Value {
    json!({
        "id": row.0,
        "name": row.1,
        "description": row.2,
        "created_at": stamp(&row.3),
    })
}
