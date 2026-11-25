use std::path::Path;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

use crate::config::Settings;
use crate::error::AppError;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY,
    username TEXT NOT NULL,
    email TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    force_pw_change INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS ix_users_email ON users (email);
CREATE UNIQUE INDEX IF NOT EXISTS ix_users_username ON users (username);

CREATE TABLE IF NOT EXISTS api_keys (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    key_hash TEXT NOT NULL UNIQUE,
    key_prefix TEXT NOT NULL,
    expires_at TEXT,
    last_used_at TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS ix_api_keys_user_id ON api_keys (user_id);

CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    updated_by INTEGER,
    FOREIGN KEY (updated_by) REFERENCES users(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS audit_logs (
    id INTEGER PRIMARY KEY,
    user_id INTEGER,
    action TEXT NOT NULL,
    resource_type TEXT,
    resource_id INTEGER,
    detail TEXT,
    ip_address TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS ix_audit_logs_action ON audit_logs (action);
CREATE INDEX IF NOT EXISTS ix_audit_logs_created_at ON audit_logs (created_at);
CREATE INDEX IF NOT EXISTS ix_audit_logs_user_id ON audit_logs (user_id);

CREATE TABLE IF NOT EXISTS maintenance_windows (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    starts_at TEXT NOT NULL,
    ends_at TEXT NOT NULL,
    repeat_cron TEXT,
    suppress_alerts INTEGER NOT NULL DEFAULT 1,
    created_by INTEGER,
    created_at TEXT NOT NULL,
    FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS monitor_groups (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    created_by INTEGER,
    created_at TEXT NOT NULL,
    FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS notification_channels (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    channel_type TEXT NOT NULL,
    config TEXT NOT NULL,
    is_enabled INTEGER NOT NULL DEFAULT 1,
    created_by INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS monitors (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    monitor_type TEXT NOT NULL DEFAULT 'http_get',
    group_id INTEGER,
    is_enabled INTEGER NOT NULL DEFAULT 1,
    check_interval_seconds INTEGER NOT NULL DEFAULT 60,
    timeout_seconds INTEGER NOT NULL DEFAULT 10,
    retry_count INTEGER NOT NULL DEFAULT 3,
    retry_interval_seconds INTEGER NOT NULL DEFAULT 20,
    alert_cooldown_seconds INTEGER NOT NULL DEFAULT 1800,
    request_headers TEXT,
    request_body TEXT,
    expected_status_codes TEXT NOT NULL DEFAULT '2xx',
    expected_body_type TEXT,
    expected_body_value TEXT,
    response_time_warning_ms INTEGER NOT NULL DEFAULT 2000,
    ssl_check_enabled INTEGER NOT NULL DEFAULT 1,
    ssl_expiry_warning_days INTEGER NOT NULL DEFAULT 30,
    alerts_enabled INTEGER NOT NULL DEFAULT 1,
    heartbeat_token TEXT,
    heartbeat_grace_seconds INTEGER NOT NULL DEFAULT 60,
    heartbeat_last_ping TEXT,
    current_status TEXT NOT NULL DEFAULT 'pending',
    last_checked_at TEXT,
    last_response_time_ms INTEGER,
    consecutive_failures INTEGER NOT NULL DEFAULT 0,
    created_by INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE SET NULL,
    FOREIGN KEY (group_id) REFERENCES monitor_groups(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS ix_monitors_current_status ON monitors (current_status);
CREATE INDEX IF NOT EXISTS ix_monitors_group_id ON monitors (group_id);
CREATE UNIQUE INDEX IF NOT EXISTS ix_monitors_heartbeat_token ON monitors (heartbeat_token);

CREATE TABLE IF NOT EXISTS check_results (
    id INTEGER PRIMARY KEY,
    monitor_id INTEGER NOT NULL,
    checked_at TEXT NOT NULL,
    status TEXT NOT NULL,
    response_time_ms INTEGER,
    status_code INTEGER,
    is_ssl_valid INTEGER,
    ssl_expiry_days INTEGER,
    error_message TEXT,
    FOREIGN KEY (monitor_id) REFERENCES monitors(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS ix_check_results_monitor_checked ON check_results (monitor_id, checked_at);

CREATE TABLE IF NOT EXISTS incidents (
    id INTEGER PRIMARY KEY,
    monitor_id INTEGER NOT NULL,
    started_at TEXT NOT NULL,
    resolved_at TEXT,
    duration_seconds INTEGER,
    root_cause TEXT,
    notification_sent INTEGER NOT NULL DEFAULT 0,
    recovery_sent INTEGER NOT NULL DEFAULT 0,
    acknowledged_by INTEGER,
    acknowledged_at TEXT,
    FOREIGN KEY (acknowledged_by) REFERENCES users(id) ON DELETE SET NULL,
    FOREIGN KEY (monitor_id) REFERENCES monitors(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS ix_incidents_monitor_resolved ON incidents (monitor_id, resolved_at);
CREATE INDEX IF NOT EXISTS ix_incidents_started_at ON incidents (started_at);

CREATE TABLE IF NOT EXISTS maintenance_window_monitors (
    window_id INTEGER NOT NULL,
    monitor_id INTEGER NOT NULL,
    PRIMARY KEY (window_id, monitor_id),
    FOREIGN KEY (monitor_id) REFERENCES monitors(id) ON DELETE CASCADE,
    FOREIGN KEY (window_id) REFERENCES maintenance_windows(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS monitor_notification_channels (
    monitor_id INTEGER NOT NULL,
    channel_id INTEGER NOT NULL,
    on_down INTEGER NOT NULL DEFAULT 1,
    on_recovery INTEGER NOT NULL DEFAULT 1,
    on_ssl_warn INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (monitor_id, channel_id),
    FOREIGN KEY (channel_id) REFERENCES notification_channels(id) ON DELETE CASCADE,
    FOREIGN KEY (monitor_id) REFERENCES monitors(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS monitor_users (
    monitor_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    notify INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (monitor_id, user_id),
    FOREIGN KEY (monitor_id) REFERENCES monitors(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
"#;

pub async fn connect(settings: &Settings) -> Result<SqlitePool, AppError> {
    crate::config::ensure_parent(&settings.database_path).map_err(|err| {
        tracing::error!(error = %err, "failed to create database directory");
        AppError::new(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Database directory is not writable",
        )
    })?;

    let options = SqliteConnectOptions::new()
        .filename(Path::new(&settings.database_path))
        .create_if_missing(true)
        .foreign_keys(true)
        .busy_timeout(std::time::Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await?;

    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA synchronous = NORMAL")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA temp_store = MEMORY")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA cache_size = -2000")
        .execute(&pool)
        .await?;

    for statement in SCHEMA.split(';') {
        let statement = statement.trim();
        if statement.is_empty() {
            continue;
        }
        sqlx::query(statement).execute(&pool).await?;
    }
    Ok(pool)
}

pub fn flag(value: i64) -> bool {
    value != 0
}

pub fn bit(value: bool) -> i64 {
    i64::from(value)
}
