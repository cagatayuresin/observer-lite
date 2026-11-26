use std::collections::BTreeMap;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::audit;
use crate::crypto::{decrypt_secret, encrypt_secret};
use crate::error::{AppError, AppResult};
use crate::extract::{ClientIp, SuperUser};
use crate::routes::users::stamp;
use crate::services::notify::send_email;
use crate::state::AppState;
use crate::timeutil::{format_db, now};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/settings", get(get_settings).put(put_settings))
        .route("/api/settings/test-smtp", axum::routing::post(test_smtp))
        .route("/api/audit-log", get(audit_log))
        .route("/api/export/monitors", get(export_monitors))
        .route("/api/import/monitors", axum::routing::post(import_monitors))
}

async fn get_settings(State(state): State<AppState>, _user: SuperUser) -> AppResult<Json<Value>> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM app_settings")
        .fetch_all(&state.db)
        .await?;
    let mut object = Map::new();
    for (key, value) in rows {
        let shown = if key == "smtp_password_enc" && !value.is_empty() {
            "***".to_string()
        } else {
            value
        };
        object.insert(key, Value::String(shown));
    }
    Ok(Json(Value::Object(object)))
}

async fn put_settings(
    State(state): State<AppState>,
    user: SuperUser,
    ClientIp(ip): ClientIp,
    Json(body): Json<Map<String, Value>>,
) -> AppResult<Json<Value>> {
    let stamp_now = format_db(now());
    for (key, value) in body {
        if key.len() > 64 {
            return Err(AppError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Setting key is too long",
            ));
        }
        let mut stored = setting_string(&value);
        if key == "smtp_password_enc" && (stored == "***" || stored.is_empty()) {
            continue;
        }
        if key == "smtp_password_enc"
            && decrypt_secret(&state.settings.secret_key, &stored).is_err()
        {
            stored = encrypt_secret(&state.settings.secret_key, &stored).unwrap_or(stored);
        }
        sqlx::query(
            "INSERT INTO app_settings (key, value, updated_at, updated_by) VALUES (?, ?, ?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at, updated_by = excluded.updated_by",
        )
        .bind(&key)
        .bind(&stored)
        .bind(&stamp_now)
        .bind(user.id)
        .execute(&state.db)
        .await?;
    }
    audit::write(
        &state.db,
        Some(user.id),
        "settings.update",
        Some("settings"),
        None,
        None,
        ip.as_deref(),
    )
    .await?;
    Ok(Json(json!({"message": "Settings updated"})))
}

async fn test_smtp(State(state): State<AppState>, _user: SuperUser) -> AppResult<Json<Value>> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT key, value FROM app_settings WHERE key IN ('smtp_host', 'smtp_port', 'smtp_user', 'smtp_password_enc', 'smtp_from')",
    )
    .fetch_all(&state.db)
    .await?;
    let map: BTreeMap<_, _> = rows.into_iter().collect();
    let from = map.get("smtp_from").cloned().unwrap_or_default();
    let port: i64 = map
        .get("smtp_port")
        .and_then(|value| value.parse().ok())
        .unwrap_or(587);
    let config = json!({
        "smtp_host": map.get("smtp_host").cloned().unwrap_or_default(),
        "smtp_port": port,
        "smtp_user": map.get("smtp_user").cloned().unwrap_or_default(),
        "smtp_password_enc": map.get("smtp_password_enc").cloned().unwrap_or_default(),
        "smtp_from": from.clone(),
        "recipients": [from],
    });
    let ok = send_email(
        &state,
        &config.to_string(),
        "Observer-Lite Test",
        "<p>Test notification from Observer-Lite.</p>",
    )
    .await;
    if !ok {
        return Err(AppError::new(
            StatusCode::BAD_GATEWAY,
            "SMTP test failed — check settings",
        ));
    }
    Ok(Json(json!({"message": "SMTP test successful"})))
}

#[derive(Deserialize)]
struct AuditQuery {
    action: Option<String>,
    user_id: Option<i64>,
    #[serde(default = "audit_limit")]
    limit: i64,
}

fn audit_limit() -> i64 {
    200
}

async fn audit_log(
    State(state): State<AppState>,
    _user: SuperUser,
    Query(query): Query<AuditQuery>,
) -> AppResult<Json<Vec<Value>>> {
    if query.limit > 1000 {
        return Err(AppError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "limit must be at most 1000",
        ));
    }
    let mut sql = String::from(
        "SELECT id, user_id, action, resource_type, resource_id, detail, ip_address, created_at FROM audit_logs WHERE 1 = 1",
    );
    let action = query.action.filter(|value| !value.is_empty());
    let user_id = query.user_id.filter(|id| *id != 0);
    if action.is_some() {
        sql.push_str(" AND action = ?");
    }
    if user_id.is_some() {
        sql.push_str(" AND user_id = ?");
    }
    sql.push_str(" ORDER BY created_at DESC LIMIT ?");
    let mut q = sqlx::query_as::<_, AuditRow>(sqlx::AssertSqlSafe(sql));
    if let Some(action) = action {
        q = q.bind(action);
    }
    if let Some(user_id) = user_id {
        q = q.bind(user_id);
    }
    let rows = q.bind(query.limit.max(1)).fetch_all(&state.db).await?;
    Ok(Json(rows.into_iter().map(audit_json).collect()))
}

#[derive(sqlx::FromRow)]
struct AuditRow {
    id: i64,
    user_id: Option<i64>,
    action: String,
    resource_type: Option<String>,
    resource_id: Option<i64>,
    detail: Option<String>,
    ip_address: Option<String>,
    created_at: String,
}

fn audit_json(row: AuditRow) -> Value {
    json!({
        "id": row.id,
        "user_id": row.user_id,
        "action": row.action,
        "resource_type": row.resource_type,
        "resource_id": row.resource_id,
        "detail": row.detail,
        "ip_address": row.ip_address,
        "created_at": stamp(&row.created_at),
    })
}

const EXPORT_KEYS: &[&str] = &[
    "name",
    "url",
    "monitor_type",
    "check_interval_seconds",
    "timeout_seconds",
    "retry_count",
    "retry_interval_seconds",
    "alert_cooldown_seconds",
    "request_headers",
    "request_body",
    "expected_status_codes",
    "expected_body_type",
    "expected_body_value",
    "response_time_warning_ms",
    "ssl_check_enabled",
    "ssl_expiry_warning_days",
    "alerts_enabled",
    "heartbeat_grace_seconds",
];

async fn export_monitors(
    State(state): State<AppState>,
    _user: SuperUser,
) -> AppResult<Json<Value>> {
    let rows: Vec<MonitorExport> = sqlx::query_as(
        "SELECT name, url, monitor_type, check_interval_seconds, timeout_seconds, retry_count,
                retry_interval_seconds, alert_cooldown_seconds, request_headers, request_body,
                expected_status_codes, expected_body_type, expected_body_value, response_time_warning_ms,
                ssl_check_enabled, ssl_expiry_warning_days, alerts_enabled, heartbeat_grace_seconds
         FROM monitors ORDER BY id",
    )
    .fetch_all(&state.db)
    .await?;
    let monitors: Vec<Value> = rows.into_iter().map(export_json).collect();
    Ok(Json(json!({
        "version": 1,
        "monitors": monitors,
        "exported_at": crate::timeutil::format_api(now()),
    })))
}

async fn import_monitors(
    State(state): State<AppState>,
    user: SuperUser,
    ClientIp(ip): ClientIp,
    Json(body): Json<Value>,
) -> AppResult<Json<Value>> {
    let items = body
        .get("monitors")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut imported = 0i64;
    for item in items {
        let Some(object) = item.as_object() else {
            continue;
        };
        let name = object
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        let url = object
            .get("url")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        if name.is_empty() || url.is_empty() {
            return Err(AppError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Each monitor needs a name and url",
            ));
        }
        let id = insert_imported(&state, user.id, object).await?;
        state.scheduler.upsert(
            state.clone(),
            id,
            int_field(object, "check_interval_seconds", 60),
        );
        imported += 1;
    }
    audit::write(
        &state.db,
        Some(user.id),
        "monitors.import",
        Some("monitor"),
        None,
        Some(&imported.to_string()),
        ip.as_deref(),
    )
    .await?;
    Ok(Json(json!({"imported": imported})))
}

fn setting_string(value: &Value) -> String {
    match value {
        Value::Null => "None".into(),
        Value::Bool(true) => "True".into(),
        Value::Bool(false) => "False".into(),
        Value::Number(number) => number.to_string(),
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

#[derive(sqlx::FromRow)]
struct MonitorExport {
    name: String,
    url: String,
    monitor_type: String,
    check_interval_seconds: i64,
    timeout_seconds: i64,
    retry_count: i64,
    retry_interval_seconds: i64,
    alert_cooldown_seconds: i64,
    request_headers: Option<String>,
    request_body: Option<String>,
    expected_status_codes: String,
    expected_body_type: Option<String>,
    expected_body_value: Option<String>,
    response_time_warning_ms: i64,
    ssl_check_enabled: i64,
    ssl_expiry_warning_days: i64,
    alerts_enabled: i64,
    heartbeat_grace_seconds: i64,
}

fn export_json(row: MonitorExport) -> Value {
    json!({
        "name": row.name,
        "url": row.url,
        "monitor_type": row.monitor_type,
        "check_interval_seconds": row.check_interval_seconds,
        "timeout_seconds": row.timeout_seconds,
        "retry_count": row.retry_count,
        "retry_interval_seconds": row.retry_interval_seconds,
        "alert_cooldown_seconds": row.alert_cooldown_seconds,
        "request_headers": row.request_headers,
        "request_body": row.request_body,
        "expected_status_codes": row.expected_status_codes,
        "expected_body_type": row.expected_body_type,
        "expected_body_value": row.expected_body_value,
        "response_time_warning_ms": row.response_time_warning_ms,
        "ssl_check_enabled": row.ssl_check_enabled != 0,
        "ssl_expiry_warning_days": row.ssl_expiry_warning_days,
        "alerts_enabled": row.alerts_enabled != 0,
        "heartbeat_grace_seconds": row.heartbeat_grace_seconds,
    })
}

async fn insert_imported(
    state: &AppState,
    user_id: i64,
    object: &Map<String, Value>,
) -> AppResult<i64> {
    let stamp_now = format_db(now());
    let monitor_type = str_field(object, "monitor_type", "http_get");
    let token = if monitor_type == "heartbeat" {
        Some(crate::crypto::generate_heartbeat_token())
    } else {
        None
    };
    let id = sqlx::query(
        "INSERT INTO monitors (
            name, url, monitor_type, is_enabled, check_interval_seconds, timeout_seconds, retry_count,
            retry_interval_seconds, alert_cooldown_seconds, request_headers, request_body, expected_status_codes,
            expected_body_type, expected_body_value, response_time_warning_ms, ssl_check_enabled,
            ssl_expiry_warning_days, alerts_enabled, heartbeat_token, heartbeat_grace_seconds,
            current_status, consecutive_failures, created_by, created_at, updated_at
         ) VALUES (?, ?, ?, 1, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'pending', 0, ?, ?, ?)",
    )
    .bind(str_field(object, "name", ""))
    .bind(str_field(object, "url", ""))
    .bind(&monitor_type)
    .bind(int_field(object, "check_interval_seconds", 60))
    .bind(int_field(object, "timeout_seconds", 10))
    .bind(int_field(object, "retry_count", 3))
    .bind(int_field(object, "retry_interval_seconds", 20))
    .bind(int_field(object, "alert_cooldown_seconds", 1800))
    .bind(optional_str(object, "request_headers"))
    .bind(optional_str(object, "request_body"))
    .bind(str_field(object, "expected_status_codes", "2xx"))
    .bind(optional_str(object, "expected_body_type"))
    .bind(optional_str(object, "expected_body_value"))
    .bind(int_field(object, "response_time_warning_ms", 2000))
    .bind(i64::from(bool_field(object, "ssl_check_enabled", true)))
    .bind(int_field(object, "ssl_expiry_warning_days", 30))
    .bind(i64::from(bool_field(object, "alerts_enabled", true)))
    .bind(token)
    .bind(int_field(object, "heartbeat_grace_seconds", 60))
    .bind(user_id)
    .bind(&stamp_now)
    .bind(&stamp_now)
    .execute(&state.db)
    .await?
    .last_insert_rowid();
    let _ = EXPORT_KEYS;
    Ok(id)
}

fn str_field(object: &Map<String, Value>, key: &str, default: &str) -> String {
    object
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or(default)
        .to_string()
}

fn optional_str(object: &Map<String, Value>, key: &str) -> Option<String> {
    object.get(key).and_then(Value::as_str).map(str::to_string)
}

fn int_field(object: &Map<String, Value>, key: &str, default: i64) -> i64 {
    object.get(key).and_then(Value::as_i64).unwrap_or(default)
}

fn bool_field(object: &Map<String, Value>, key: &str, default: bool) -> bool {
    object.get(key).and_then(Value::as_bool).unwrap_or(default)
}
