use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::audit;
use crate::crypto::generate_api_key;
use crate::error::{AppError, AppResult};
use crate::extract::{AuthUser, ClientIp};
use crate::routes::users::stamp;
use crate::state::AppState;
use crate::timeutil::{format_db, now, parse_db};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/api-keys", get(list).post(create))
        .route("/api/api-keys/{key_id}", axum::routing::delete(remove))
}

#[derive(Deserialize)]
struct KeyBody {
    name: String,
    #[serde(default)]
    expires_at: Option<String>,
}

async fn list(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<Vec<Value>>> {
    let rows: Vec<KeyRow> = if user.is_super() {
        sqlx::query_as(
            "SELECT id, name, key_prefix, expires_at, last_used_at, created_at FROM api_keys ORDER BY created_at DESC",
        )
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as(
            "SELECT id, name, key_prefix, expires_at, last_used_at, created_at FROM api_keys WHERE user_id = ? ORDER BY created_at DESC",
        )
        .bind(user.id)
        .fetch_all(&state.db)
        .await?
    };
    Ok(Json(rows.into_iter().map(key_json).collect()))
}

async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    ClientIp(ip): ClientIp,
    Json(body): Json<KeyBody>,
) -> AppResult<(StatusCode, Json<Value>)> {
    if body.name.trim().is_empty() {
        return Err(AppError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Name is required",
        ));
    }
    let expires = match &body.expires_at {
        Some(value) => Some(format_db(parse_db(value).ok_or_else(|| {
            AppError::new(StatusCode::UNPROCESSABLE_ENTITY, "Invalid datetime")
        })?)),
        None => None,
    };
    let (raw, hash, prefix) = generate_api_key();
    let stamp_now = format_db(now());
    let id = sqlx::query(
        "INSERT INTO api_keys (user_id, name, key_hash, key_prefix, expires_at, created_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(user.id)
    .bind(body.name.trim())
    .bind(&hash)
    .bind(&prefix)
    .bind(&expires)
    .bind(&stamp_now)
    .execute(&state.db)
    .await?
    .last_insert_rowid();
    audit::write(
        &state.db,
        Some(user.id),
        "api_key.create",
        Some("api_key"),
        Some(id),
        Some(&prefix),
        ip.as_deref(),
    )
    .await?;
    let mut value = key_json(KeyRow {
        id,
        name: body.name,
        key_prefix: prefix,
        expires_at: expires,
        last_used_at: None,
        created_at: stamp_now,
    });
    value["raw_key"] = json!(raw);
    Ok((StatusCode::CREATED, Json(value)))
}

async fn remove(
    State(state): State<AppState>,
    user: AuthUser,
    ClientIp(ip): ClientIp,
    Path(key_id): Path<i64>,
) -> AppResult<StatusCode> {
    let row: Option<(i64,)> = sqlx::query_as("SELECT user_id FROM api_keys WHERE id = ?")
        .bind(key_id)
        .fetch_optional(&state.db)
        .await?;
    let Some((owner,)) = row else {
        return Err(AppError::new(StatusCode::NOT_FOUND, "API key not found"));
    };
    if owner != user.id && !user.is_super() {
        return Err(AppError::new(
            StatusCode::FORBIDDEN,
            "Cannot delete another user's API key",
        ));
    }
    sqlx::query("DELETE FROM api_keys WHERE id = ?")
        .bind(key_id)
        .execute(&state.db)
        .await?;
    audit::write(
        &state.db,
        Some(user.id),
        "api_key.delete",
        Some("api_key"),
        Some(key_id),
        None,
        ip.as_deref(),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(sqlx::FromRow)]
struct KeyRow {
    id: i64,
    name: String,
    key_prefix: String,
    expires_at: Option<String>,
    last_used_at: Option<String>,
    created_at: String,
}

fn key_json(row: KeyRow) -> Value {
    json!({
        "id": row.id,
        "name": row.name,
        "key_prefix": row.key_prefix,
        "expires_at": row.expires_at.as_deref().map(stamp),
        "last_used_at": row.last_used_at.as_deref().map(stamp),
        "created_at": stamp(&row.created_at),
    })
}
