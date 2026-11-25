use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::{
    create_access_token, create_refresh_token, decode_token, hash_password_async,
    verify_password_async,
};
use crate::error::{AppError, AppResult};
use crate::extract::{AuthUser, ClientIp};
use crate::state::AppState;
use crate::timeutil::{format_db, now};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/auth/refresh", post(refresh))
        .route("/api/auth/me", get(me))
        .route("/api/auth/change-password", post(change_password))
}

#[derive(Deserialize)]
struct LoginBody {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct RefreshBody {
    refresh_token: String,
}

#[derive(Deserialize)]
struct PasswordBody {
    current_password: String,
    new_password: String,
}

async fn login(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Json(body): Json<LoginBody>,
) -> AppResult<Json<Value>> {
    let key = ip.unwrap_or_else(|| "unknown".into());
    if !state
        .login_limiter
        .allow(&key, state.settings.login_rate_limit)
    {
        return Err(AppError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "Too many login attempts",
        ));
    }
    let row: Option<(i64, String, String, i64)> =
        sqlx::query_as("SELECT id, password_hash, role, is_active FROM users WHERE username = ?")
            .bind(&body.username)
            .fetch_optional(&state.db)
            .await?;
    let Some((id, hash, role, active)) = row else {
        return Err(AppError::new(
            StatusCode::UNAUTHORIZED,
            "Invalid credentials",
        ));
    };
    if !verify_password_async(body.password, hash).await {
        return Err(AppError::new(
            StatusCode::UNAUTHORIZED,
            "Invalid credentials",
        ));
    }
    if active == 0 {
        return Err(AppError::new(StatusCode::FORBIDDEN, "Account disabled"));
    }
    Ok(Json(tokens(&state, id, &role)?))
}

async fn refresh(
    State(state): State<AppState>,
    Json(body): Json<RefreshBody>,
) -> AppResult<Json<Value>> {
    let claims = decode_token(&state.settings, &body.refresh_token)
        .map_err(|_| AppError::new(StatusCode::UNAUTHORIZED, "Invalid refresh token"))?;
    if claims.typ != "refresh" {
        return Err(AppError::new(
            StatusCode::UNAUTHORIZED,
            "Invalid token type",
        ));
    }
    let user_id: i64 = claims
        .sub
        .parse()
        .map_err(|_| AppError::new(StatusCode::UNAUTHORIZED, "Invalid refresh token"))?;
    let row: Option<(String, i64)> =
        sqlx::query_as("SELECT role, is_active FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_optional(&state.db)
            .await?;
    let Some((role, active)) = row else {
        return Err(AppError::new(
            StatusCode::UNAUTHORIZED,
            "User not found or inactive",
        ));
    };
    if active == 0 {
        return Err(AppError::new(
            StatusCode::UNAUTHORIZED,
            "User not found or inactive",
        ));
    }
    Ok(Json(tokens(&state, user_id, &role)?))
}

async fn me(user: AuthUser) -> Json<Value> {
    Json(json!({
        "id": user.id,
        "username": user.username,
        "email": user.email,
        "role": user.role,
        "force_pw_change": user.force_pw_change,
    }))
}

async fn change_password(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<PasswordBody>,
) -> AppResult<StatusCode> {
    if !verify_password_async(body.current_password, user.password_hash).await {
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            "Current password is incorrect",
        ));
    }
    if body.new_password.chars().count() < 8 {
        return Err(AppError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Password must be at least 8 characters",
        ));
    }
    let hash = hash_password_async(body.new_password).await?;
    sqlx::query(
        "UPDATE users SET password_hash = ?, force_pw_change = 0, updated_at = ? WHERE id = ?",
    )
    .bind(hash)
    .bind(format_db(now()))
    .bind(user.id)
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

fn tokens(state: &AppState, id: i64, role: &str) -> AppResult<Value> {
    Ok(json!({
        "access_token": create_access_token(&state.settings, id, role)?,
        "refresh_token": create_refresh_token(&state.settings, id)?,
        "token_type": "bearer",
    }))
}
