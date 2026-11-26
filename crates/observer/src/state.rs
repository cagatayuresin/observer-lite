use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::Semaphore;

use crate::auth::{hash_password_async, LoginLimiter};
use crate::config::Settings;
use crate::crypto::hash_password;
use crate::db::connect;
use crate::error::AppError;
use crate::scheduler::Scheduler;
use crate::ssehub::SseHub;
use crate::timeutil::{format_db, now};

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub settings: Settings,
    pub http: reqwest::Client,
    pub sse: SseHub,
    pub scheduler: Arc<Scheduler>,
    pub checks: Arc<Semaphore>,
    pub login_limiter: Arc<LoginLimiter>,
}

pub async fn bootstrap(settings: Settings) -> Result<AppState, AppError> {
    if settings.secret_key.starts_with("change-me") {
        tracing::warn!("SECRET_KEY is the default value. Set a long random SECRET_KEY before exposing this service.");
    }
    let db = connect(&settings).await?;
    seed(&db, settings.port).await?;

    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .danger_accept_invalid_certs(true)
        .pool_max_idle_per_host(20)
        .build()
        .map_err(|err| {
            tracing::error!(error = %err, "http client");
            AppError::new(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "HTTP client failed to start",
            )
        })?;

    let scheduler = Arc::new(Scheduler::new(settings.scheduler_enabled));
    let state = AppState {
        db,
        checks: Arc::new(Semaphore::new(settings.max_concurrent_checks)),
        settings,
        http,
        sse: SseHub::default(),
        scheduler,
        login_limiter: Arc::new(LoginLimiter::default()),
    };

    if state.settings.scheduler_enabled {
        let rows: Vec<(i64, i64)> =
            sqlx::query_as("SELECT id, check_interval_seconds FROM monitors WHERE is_enabled = 1")
                .fetch_all(&state.db)
                .await?;
        for (id, interval) in rows {
            state.scheduler.upsert(state.clone(), id, interval);
        }
        state.scheduler.spawn_daily(state.clone());
    }
    Ok(state)
}

async fn seed(pool: &SqlitePool, port: u16) -> Result<(), AppError> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    if count.0 > 0 {
        return Ok(());
    }
    let now = format_db(now());
    let password_hash = hash_password_async("admin".to_string())
        .await
        .or_else(|_| {
            hash_password("admin").map_err(|_| {
                AppError::new(
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "Password hashing failed",
                )
            })
        })?;
    sqlx::query(
        "INSERT INTO users (username, email, password_hash, role, is_active, force_pw_change, created_at, updated_at)
         VALUES ('admin', 'admin@localhost', ?, 'superadmin', 1, 1, ?, ?)",
    )
    .bind(password_hash)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    for (key, value) in [
        ("data_retention_days", "90".to_string()),
        ("app_base_url", format!("http://localhost:{port}")),
    ] {
        sqlx::query("INSERT INTO app_settings (key, value, updated_at) VALUES (?, ?, ?)")
            .bind(key)
            .bind(value)
            .bind(&now)
            .execute(pool)
            .await?;
    }
    tracing::info!("first run: superadmin admin / admin was created and must change the password");
    Ok(())
}
