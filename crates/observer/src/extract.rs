use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::http::{HeaderMap, StatusCode};
use std::net::SocketAddr;

use crate::auth::decode_token;
use crate::crypto::hash_api_key;
use crate::db::flag;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::timeutil::{format_db, now};

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub role: String,
    pub is_active: bool,
    pub force_pw_change: bool,
    pub password_hash: String,
}

impl AuthUser {
    pub fn is_privileged(&self) -> bool {
        self.role == "admin" || self.role == "superadmin"
    }

    pub fn is_super(&self) -> bool {
        self.role == "superadmin"
    }
}

pub struct AdminUser(pub AuthUser);
pub struct SuperUser(pub AuthUser);

impl std::ops::Deref for AdminUser {
    type Target = AuthUser;
    fn deref(&self) -> &AuthUser {
        &self.0
    }
}

impl std::ops::Deref for SuperUser {
    type Target = AuthUser;
    fn deref(&self) -> &AuthUser {
        &self.0
    }
}

pub struct ClientIp(pub Option<String>);

const CREDENTIALS: &str = "Could not validate credentials";

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = bearer(&parts.headers).ok_or_else(unauthorized)?;
        if let Some(raw) = token.strip_prefix("obs_") {
            let _ = raw;
            return user_from_api_key(state, &token).await;
        }
        let claims = decode_token(&state.settings, &token).map_err(|_| unauthorized())?;
        if claims.typ != "access" {
            return Err(unauthorized());
        }
        let user_id: i64 = claims.sub.parse().map_err(|_| unauthorized())?;
        load_active_user(&state.db, user_id).await
    }
}

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if !user.is_privileged() {
            return Err(AppError::new(
                StatusCode::FORBIDDEN,
                "Insufficient permissions",
            ));
        }
        Ok(AdminUser(user))
    }
}

impl FromRequestParts<AppState> for SuperUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if !user.is_super() {
            return Err(AppError::new(
                StatusCode::FORBIDDEN,
                "Insufficient permissions",
            ));
        }
        Ok(SuperUser(user))
    }
}

impl FromRequestParts<AppState> for ClientIp {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let ip = parts
            .extensions
            .get::<axum::extract::ConnectInfo<SocketAddr>>()
            .map(|info| info.0.ip().to_string());
        Ok(ClientIp(ip))
    }
}

fn unauthorized() -> AppError {
    AppError::new(StatusCode::UNAUTHORIZED, CREDENTIALS)
}

fn bearer(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let token = value.strip_prefix("Bearer ")?.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

async fn user_from_api_key(state: &AppState, raw: &str) -> AppResult<AuthUser> {
    let hash = hash_api_key(raw);
    let row: Option<(i64,)> = sqlx::query_as("SELECT user_id FROM api_keys WHERE key_hash = ?")
        .bind(&hash)
        .fetch_optional(&state.db)
        .await?;
    let Some((user_id,)) = row else {
        return Err(unauthorized());
    };
    sqlx::query("UPDATE api_keys SET last_used_at = ? WHERE key_hash = ?")
        .bind(format_db(now()))
        .bind(&hash)
        .execute(&state.db)
        .await?;
    load_active_user(&state.db, user_id).await
}

async fn load_active_user(pool: &sqlx::SqlitePool, id: i64) -> AppResult<AuthUser> {
    let row: Option<(i64, String, String, String, String, i64, i64)> = sqlx::query_as(
        "SELECT id, username, email, password_hash, role, is_active, force_pw_change FROM users WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    let Some((id, username, email, password_hash, role, is_active, force_pw_change)) = row else {
        return Err(unauthorized());
    };
    if !flag(is_active) {
        return Err(unauthorized());
    }
    Ok(AuthUser {
        id,
        username,
        email,
        role,
        is_active: true,
        force_pw_change: flag(force_pw_change),
        password_hash,
    })
}
