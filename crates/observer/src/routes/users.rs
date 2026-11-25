use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::audit;
use crate::auth::hash_password_async;
use crate::error::{AppError, AppResult};
use crate::extract::{ClientIp, SuperUser};
use crate::state::AppState;
use crate::timeutil::{format_api, format_db, now, parse_db};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/users", get(list).post(create))
        .route(
            "/api/users/{user_id}",
            get(get_one).put(update).delete(remove),
        )
        .route("/api/users/{user_id}/reset-password", post(reset_password))
}

#[derive(Deserialize)]
struct UserCreate {
    username: String,
    email: String,
    password: String,
    #[serde(default = "viewer")]
    role: String,
}

fn viewer() -> String {
    "viewer".into()
}

#[derive(Deserialize)]
struct UserUpdate {
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    is_active: Option<bool>,
}

async fn list(State(state): State<AppState>, _user: SuperUser) -> AppResult<Json<Vec<Value>>> {
    let rows: Vec<(i64, String, String, String, i64, i64, String)> = sqlx::query_as(
        "SELECT id, username, email, role, is_active, force_pw_change, created_at
         FROM users ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows.into_iter().map(user_json).collect()))
}

async fn create(
    State(state): State<AppState>,
    user: SuperUser,
    ClientIp(ip): ClientIp,
    Json(body): Json<UserCreate>,
) -> AppResult<(StatusCode, Json<Value>)> {
    if body.role.len() > 16 || body.username.is_empty() || !valid_email(&body.email) {
        return Err(AppError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Invalid user",
        ));
    }
    let exists: Option<(i64,)> = sqlx::query_as("SELECT id FROM users WHERE username = ?")
        .bind(&body.username)
        .fetch_optional(&state.db)
        .await?;
    if exists.is_some() {
        return Err(AppError::new(
            StatusCode::CONFLICT,
            "Username already exists",
        ));
    }
    let stamp = format_db(now());
    let hash = hash_password_async(body.password).await?;
    let id = sqlx::query(
        "INSERT INTO users (username, email, password_hash, role, is_active, force_pw_change, created_at, updated_at)
         VALUES (?, ?, ?, ?, 1, 1, ?, ?)",
    )
    .bind(&body.username)
    .bind(&body.email)
    .bind(hash)
    .bind(&body.role)
    .bind(&stamp)
    .bind(&stamp)
    .execute(&state.db)
    .await?
    .last_insert_rowid();
    audit::write(
        &state.db,
        Some(user.id),
        "user.create",
        Some("user"),
        Some(id),
        Some(&body.username),
        ip.as_deref(),
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": id,
            "username": body.username,
            "email": body.email,
            "role": body.role,
            "is_active": true,
            "force_pw_change": true,
            "created_at": format_api(parse_db(&stamp).unwrap_or_else(now)),
        })),
    ))
}

async fn get_one(
    State(state): State<AppState>,
    _user: SuperUser,
    Path(user_id): Path<i64>,
) -> AppResult<Json<Value>> {
    let row = fetch_user(&state, user_id).await?;
    Ok(Json(user_json(row)))
}

async fn update(
    State(state): State<AppState>,
    actor: SuperUser,
    ClientIp(ip): ClientIp,
    Path(user_id): Path<i64>,
    Json(body): Json<UserUpdate>,
) -> AppResult<Json<Value>> {
    let _ = fetch_user(&state, user_id).await?;
    if let Some(email) = &body.email {
        if !valid_email(email) {
            return Err(AppError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Invalid email",
            ));
        }
        sqlx::query("UPDATE users SET email = ? WHERE id = ?")
            .bind(email)
            .bind(user_id)
            .execute(&state.db)
            .await?;
    }
    if let Some(role) = &body.role {
        if role.len() > 16 {
            return Err(AppError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Invalid role",
            ));
        }
        sqlx::query("UPDATE users SET role = ? WHERE id = ?")
            .bind(role)
            .bind(user_id)
            .execute(&state.db)
            .await?;
    }
    if let Some(active) = body.is_active {
        sqlx::query("UPDATE users SET is_active = ? WHERE id = ?")
            .bind(i64::from(active))
            .bind(user_id)
            .execute(&state.db)
            .await?;
    }
    sqlx::query("UPDATE users SET updated_at = ? WHERE id = ?")
        .bind(format_db(now()))
        .bind(user_id)
        .execute(&state.db)
        .await?;
    audit::write(
        &state.db,
        Some(actor.id),
        "user.update",
        Some("user"),
        Some(user_id),
        None,
        ip.as_deref(),
    )
    .await?;
    Ok(Json(user_json(fetch_user(&state, user_id).await?)))
}

async fn remove(
    State(state): State<AppState>,
    actor: SuperUser,
    ClientIp(ip): ClientIp,
    Path(user_id): Path<i64>,
) -> AppResult<StatusCode> {
    if user_id == actor.id {
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            "Cannot delete yourself",
        ));
    }
    let _ = fetch_user(&state, user_id).await?;
    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(user_id)
        .execute(&state.db)
        .await?;
    audit::write(
        &state.db,
        Some(actor.id),
        "user.delete",
        Some("user"),
        Some(user_id),
        None,
        ip.as_deref(),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn reset_password(
    State(state): State<AppState>,
    actor: SuperUser,
    ClientIp(ip): ClientIp,
    Path(user_id): Path<i64>,
) -> AppResult<StatusCode> {
    let _ = fetch_user(&state, user_id).await?;
    let hash = hash_password_async("password123".into()).await?;
    sqlx::query(
        "UPDATE users SET password_hash = ?, force_pw_change = 1, updated_at = ? WHERE id = ?",
    )
    .bind(hash)
    .bind(format_db(now()))
    .bind(user_id)
    .execute(&state.db)
    .await?;
    audit::write(
        &state.db,
        Some(actor.id),
        "user.reset_password",
        Some("user"),
        Some(user_id),
        None,
        ip.as_deref(),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn fetch_user(
    state: &AppState,
    id: i64,
) -> AppResult<(i64, String, String, String, i64, i64, String)> {
    sqlx::query_as(
        "SELECT id, username, email, role, is_active, force_pw_change, created_at FROM users WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::new(StatusCode::NOT_FOUND, "User not found"))
}

fn user_json(row: (i64, String, String, String, i64, i64, String)) -> Value {
    json!({
        "id": row.0,
        "username": row.1,
        "email": row.2,
        "role": row.3,
        "is_active": row.4 != 0,
        "force_pw_change": row.5 != 0,
        "created_at": stamp(&row.6),
    })
}

pub fn stamp(value: &str) -> String {
    parse_db(value)
        .map(format_api)
        .unwrap_or_else(|| value.to_string())
}

fn valid_email(value: &str) -> bool {
    let mut parts = value.split('@');
    let local = parts.next().unwrap_or("");
    let domain = parts.next().unwrap_or("");
    parts.next().is_none()
        && !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
}
