use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::audit;
use crate::checkers::matchers::parse_status_expression;
use crate::crypto::generate_heartbeat_token;
use crate::error::{AppError, AppResult};
use crate::extract::{AdminUser, AuthUser, ClientIp};
use crate::routes::users::stamp;
use crate::services::checks::run_monitor_check;
use crate::state::AppState;
use crate::timeutil::{format_db, now};

const TYPES: &[&str] = &["http_get", "http_post", "http_head", "ping", "heartbeat"];
const BODY_TYPES: &[&str] = &["contains", "equals", "not_equals"];

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/monitors", get(list).post(create))
        .route("/api/monitors/bulk", post(bulk))
        .route(
            "/api/monitors/{monitor_id}",
            get(get_one).put(update).patch(patch).delete(remove),
        )
        .route("/api/monitors/{monitor_id}/pause", post(pause))
        .route("/api/monitors/{monitor_id}/resume", post(resume))
        .route("/api/monitors/{monitor_id}/check-now", post(check_now))
        .route(
            "/api/monitors/{monitor_id}/users",
            get(list_users).post(assign_user),
        )
        .route(
            "/api/monitors/{monitor_id}/users/{user_id}",
            axum::routing::delete(unassign_user),
        )
        .route(
            "/api/monitors/{monitor_id}/channels",
            get(list_channels).post(assign_channel),
        )
        .route(
            "/api/monitors/{monitor_id}/channels/{channel_id}",
            axum::routing::delete(unassign_channel),
        )
}

#[derive(Deserialize)]
struct ListQuery {
    status: Option<String>,
    group_id: Option<i64>,
    search: Option<String>,
}

#[derive(Deserialize)]
struct AssignUserQuery {
    user_id: i64,
    #[serde(default = "yes")]
    notify: bool,
}

fn yes() -> bool {
    true
}

#[derive(Deserialize)]
struct AssignChannel {
    channel_id: i64,
    #[serde(default = "yes")]
    on_down: bool,
    #[serde(default = "yes")]
    on_recovery: bool,
    #[serde(default = "yes")]
    on_ssl_warn: bool,
}

#[derive(Deserialize)]
struct BulkBody {
    action: String,
    ids: Vec<i64>,
}

async fn list(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<Vec<Value>>> {
    let mut sql = format!("SELECT {COLS} FROM monitors WHERE 1 = 1");
    let privileged = user.is_privileged();
    if !privileged {
        sql.push_str(" AND (created_by = ? OR id IN (SELECT monitor_id FROM monitor_users WHERE user_id = ?))");
    }
    let status = query.status.filter(|value| !value.is_empty());
    let group_id = query.group_id.filter(|id| *id != 0);
    let search = query.search.filter(|value| !value.is_empty());
    if status.is_some() {
        sql.push_str(" AND current_status = ?");
    }
    if group_id.is_some() {
        sql.push_str(" AND group_id = ?");
    }
    if search.is_some() {
        sql.push_str(" AND name LIKE '%' || ? || '%'");
    }
    sql.push_str(" ORDER BY name ASC");
    let mut q = sqlx::query_as::<_, MonitorRow>(sqlx::AssertSqlSafe(sql));
    if !privileged {
        q = q.bind(user.id).bind(user.id);
    }
    if let Some(status) = status {
        q = q.bind(status);
    }
    if let Some(group_id) = group_id {
        q = q.bind(group_id);
    }
    if let Some(search) = search {
        q = q.bind(search);
    }
    let rows = q.fetch_all(&state.db).await?;
    Ok(Json(
        rows.into_iter()
            .map(|row| {
                let hide_token = !privileged && row.created_by != Some(user.id);
                monitor_json(row, hide_token)
            })
            .collect(),
    ))
}

async fn create(
    State(state): State<AppState>,
    user: AdminUser,
    ClientIp(ip): ClientIp,
    Json(body): Json<Value>,
) -> AppResult<(StatusCode, Json<Value>)> {
    let fields = normalize_create(&body)?;
    let stamp_now = format_db(now());
    let token = if fields.monitor_type == "heartbeat" {
        Some(generate_heartbeat_token())
    } else {
        None
    };
    let id = sqlx::query(
        "INSERT INTO monitors (
            name, url, monitor_type, group_id, is_enabled, check_interval_seconds, timeout_seconds,
            retry_count, retry_interval_seconds, alert_cooldown_seconds, request_headers, request_body,
            expected_status_codes, expected_body_type, expected_body_value, response_time_warning_ms,
            ssl_check_enabled, ssl_expiry_warning_days, alerts_enabled, heartbeat_token,
            heartbeat_grace_seconds, current_status, consecutive_failures, created_by, created_at, updated_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'pending', 0, ?, ?, ?)",
    )
    .bind(&fields.name)
    .bind(&fields.url)
    .bind(&fields.monitor_type)
    .bind(fields.group_id)
    .bind(i64::from(fields.is_enabled))
    .bind(fields.check_interval_seconds)
    .bind(fields.timeout_seconds)
    .bind(fields.retry_count)
    .bind(fields.retry_interval_seconds)
    .bind(fields.alert_cooldown_seconds)
    .bind(&fields.request_headers)
    .bind(&fields.request_body)
    .bind(&fields.expected_status_codes)
    .bind(&fields.expected_body_type)
    .bind(&fields.expected_body_value)
    .bind(fields.response_time_warning_ms)
    .bind(i64::from(fields.ssl_check_enabled))
    .bind(fields.ssl_expiry_warning_days)
    .bind(i64::from(fields.alerts_enabled))
    .bind(&token)
    .bind(fields.heartbeat_grace_seconds)
    .bind(user.id)
    .bind(&stamp_now)
    .bind(&stamp_now)
    .execute(&state.db)
    .await?
    .last_insert_rowid();
    sync_job(&state, id).await?;
    audit::write(
        &state.db,
        Some(user.id),
        "monitor.create",
        Some("monitor"),
        Some(id),
        Some(&fields.name),
        ip.as_deref(),
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(monitor_json(load(&state, id).await?, false)),
    ))
}

async fn get_one(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let row = load_visible(&state, &user, id).await?;
    let hide = !user.is_privileged() && row.created_by != Some(user.id);
    Ok(Json(monitor_json(row, hide)))
}

async fn update(
    State(state): State<AppState>,
    user: AdminUser,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
    Json(body): Json<Map<String, Value>>,
) -> AppResult<Json<Value>> {
    let _ = load(&state, id).await?;
    apply_changes(&state, id, &body, false).await?;
    sync_job(&state, id).await?;
    audit::write(
        &state.db,
        Some(user.id),
        "monitor.update",
        Some("monitor"),
        Some(id),
        None,
        ip.as_deref(),
    )
    .await?;
    Ok(Json(monitor_json(load(&state, id).await?, false)))
}

async fn patch(
    State(state): State<AppState>,
    user: AdminUser,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
    Json(body): Json<Map<String, Value>>,
) -> AppResult<Json<Value>> {
    let _ = load(&state, id).await?;
    apply_changes(&state, id, &body, true).await?;
    sync_job(&state, id).await?;
    audit::write(
        &state.db,
        Some(user.id),
        "monitor.patch",
        Some("monitor"),
        Some(id),
        None,
        ip.as_deref(),
    )
    .await?;
    Ok(Json(monitor_json(load(&state, id).await?, false)))
}

async fn remove(
    State(state): State<AppState>,
    user: AdminUser,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    let _ = load(&state, id).await?;
    state.scheduler.remove(id);
    sqlx::query("DELETE FROM monitors WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;
    audit::write(
        &state.db,
        Some(user.id),
        "monitor.delete",
        Some("monitor"),
        Some(id),
        None,
        ip.as_deref(),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn pause(
    State(state): State<AppState>,
    user: AdminUser,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    let _ = load(&state, id).await?;
    sqlx::query("UPDATE monitors SET is_enabled = 0, current_status = 'paused', updated_at = ? WHERE id = ?")
        .bind(format_db(now()))
        .bind(id)
        .execute(&state.db)
        .await?;
    state.scheduler.pause(id);
    audit::write(
        &state.db,
        Some(user.id),
        "monitor.pause",
        Some("monitor"),
        Some(id),
        None,
        ip.as_deref(),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn resume(
    State(state): State<AppState>,
    user: AdminUser,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    let _ = load(&state, id).await?;
    sqlx::query("UPDATE monitors SET is_enabled = 1, current_status = 'pending', updated_at = ? WHERE id = ?")
        .bind(format_db(now()))
        .bind(id)
        .execute(&state.db)
        .await?;
    sync_job(&state, id).await?;
    audit::write(
        &state.db,
        Some(user.id),
        "monitor.resume",
        Some("monitor"),
        Some(id),
        None,
        ip.as_deref(),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn check_now(
    State(state): State<AppState>,
    _user: AdminUser,
    Path(id): Path<i64>,
) -> AppResult<(StatusCode, Json<Value>)> {
    let _ = load(&state, id).await?;
    let app = state.clone();
    tokio::spawn(async move {
        run_monitor_check(&app, id).await;
    });
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({"message": "Check triggered"})),
    ))
}

async fn bulk(
    State(state): State<AppState>,
    user: AdminUser,
    ClientIp(ip): ClientIp,
    Json(body): Json<BulkBody>,
) -> AppResult<StatusCode> {
    for id in body.ids {
        let exists: Option<(i64,)> = sqlx::query_as("SELECT id FROM monitors WHERE id = ?")
            .bind(id)
            .fetch_optional(&state.db)
            .await?;
        if exists.is_none() {
            continue;
        }
        match body.action.as_str() {
            "enable" => {
                sqlx::query("UPDATE monitors SET is_enabled = 1, updated_at = ? WHERE id = ?")
                    .bind(format_db(now()))
                    .bind(id)
                    .execute(&state.db)
                    .await?;
                sync_job(&state, id).await?;
            }
            "disable" => {
                sqlx::query("UPDATE monitors SET is_enabled = 0, updated_at = ? WHERE id = ?")
                    .bind(format_db(now()))
                    .bind(id)
                    .execute(&state.db)
                    .await?;
                state.scheduler.pause(id);
            }
            "delete" => {
                state.scheduler.remove(id);
                sqlx::query("DELETE FROM monitors WHERE id = ?")
                    .bind(id)
                    .execute(&state.db)
                    .await?;
            }
            _ => {}
        }
    }
    audit::write(
        &state.db,
        Some(user.id),
        "monitor.bulk",
        Some("monitor"),
        None,
        Some(&body.action),
        ip.as_deref(),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_users(
    State(state): State<AppState>,
    _user: AdminUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Vec<Value>>> {
    let rows: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT user_id, notify FROM monitor_users WHERE monitor_id = ? ORDER BY user_id",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|(user_id, notify)| json!({"user_id": user_id, "notify": notify != 0}))
            .collect(),
    ))
}

async fn assign_user(
    State(state): State<AppState>,
    user: AdminUser,
    Path(monitor_id): Path<i64>,
    Query(query): Query<AssignUserQuery>,
) -> AppResult<(StatusCode, Json<Value>)> {
    let exists: Option<(i64,)> =
        sqlx::query_as("SELECT user_id FROM monitor_users WHERE monitor_id = ? AND user_id = ?")
            .bind(monitor_id)
            .bind(query.user_id)
            .fetch_optional(&state.db)
            .await?;
    if exists.is_some() {
        return Err(AppError::new(StatusCode::CONFLICT, "User already assigned"));
    }
    sqlx::query("INSERT INTO monitor_users (monitor_id, user_id, notify) VALUES (?, ?, ?)")
        .bind(monitor_id)
        .bind(query.user_id)
        .bind(i64::from(query.notify))
        .execute(&state.db)
        .await?;
    audit::write(
        &state.db,
        Some(user.id),
        "monitor.assign_user",
        Some("monitor"),
        Some(monitor_id),
        None,
        None,
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({"monitor_id": monitor_id, "user_id": query.user_id})),
    ))
}

async fn unassign_user(
    State(state): State<AppState>,
    _user: AdminUser,
    Path((monitor_id, user_id)): Path<(i64, i64)>,
) -> AppResult<StatusCode> {
    sqlx::query("DELETE FROM monitor_users WHERE monitor_id = ? AND user_id = ?")
        .bind(monitor_id)
        .bind(user_id)
        .execute(&state.db)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_channels(
    State(state): State<AppState>,
    _user: AdminUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Vec<Value>>> {
    let rows: Vec<(i64, i64, i64, i64)> = sqlx::query_as(
        "SELECT channel_id, on_down, on_recovery, on_ssl_warn FROM monitor_notification_channels WHERE monitor_id = ? ORDER BY channel_id",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|(channel_id, on_down, on_recovery, on_ssl_warn)| {
                json!({
                    "channel_id": channel_id,
                    "on_down": on_down != 0,
                    "on_recovery": on_recovery != 0,
                    "on_ssl_warn": on_ssl_warn != 0,
                })
            })
            .collect(),
    ))
}

async fn assign_channel(
    State(state): State<AppState>,
    user: AdminUser,
    Path(monitor_id): Path<i64>,
    Json(body): Json<AssignChannel>,
) -> AppResult<(StatusCode, Json<Value>)> {
    let exists: Option<(i64,)> = sqlx::query_as(
        "SELECT channel_id FROM monitor_notification_channels WHERE monitor_id = ? AND channel_id = ?",
    )
    .bind(monitor_id)
    .bind(body.channel_id)
    .fetch_optional(&state.db)
    .await?;
    if exists.is_some() {
        return Err(AppError::new(
            StatusCode::CONFLICT,
            "Channel already assigned",
        ));
    }
    sqlx::query(
        "INSERT INTO monitor_notification_channels (monitor_id, channel_id, on_down, on_recovery, on_ssl_warn) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(monitor_id)
    .bind(body.channel_id)
    .bind(i64::from(body.on_down))
    .bind(i64::from(body.on_recovery))
    .bind(i64::from(body.on_ssl_warn))
    .execute(&state.db)
    .await?;
    audit::write(
        &state.db,
        Some(user.id),
        "monitor.assign_channel",
        Some("monitor"),
        Some(monitor_id),
        None,
        None,
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({"monitor_id": monitor_id, "channel_id": body.channel_id})),
    ))
}

async fn unassign_channel(
    State(state): State<AppState>,
    _user: AdminUser,
    Path((monitor_id, channel_id)): Path<(i64, i64)>,
) -> AppResult<StatusCode> {
    sqlx::query(
        "DELETE FROM monitor_notification_channels WHERE monitor_id = ? AND channel_id = ?",
    )
    .bind(monitor_id)
    .bind(channel_id)
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

const COLS: &str = "id, name, url, monitor_type, group_id, is_enabled, check_interval_seconds, timeout_seconds, retry_count, retry_interval_seconds, alert_cooldown_seconds, request_headers, request_body, expected_status_codes, expected_body_type, expected_body_value, response_time_warning_ms, ssl_check_enabled, ssl_expiry_warning_days, alerts_enabled, heartbeat_token, heartbeat_grace_seconds, current_status, last_checked_at, last_response_time_ms, consecutive_failures, created_by, created_at, updated_at";

#[derive(sqlx::FromRow)]
struct MonitorRow {
    id: i64,
    name: String,
    url: String,
    monitor_type: String,
    group_id: Option<i64>,
    is_enabled: i64,
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
    heartbeat_token: Option<String>,
    heartbeat_grace_seconds: i64,
    current_status: String,
    last_checked_at: Option<String>,
    last_response_time_ms: Option<i64>,
    consecutive_failures: i64,
    created_by: Option<i64>,
    created_at: String,
    updated_at: String,
}

struct Fields {
    name: String,
    url: String,
    monitor_type: String,
    group_id: Option<i64>,
    is_enabled: bool,
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
    ssl_check_enabled: bool,
    ssl_expiry_warning_days: i64,
    alerts_enabled: bool,
    heartbeat_grace_seconds: i64,
}

fn normalize_create(body: &Value) -> AppResult<Fields> {
    let name = body
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let url = body
        .get("url")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if name.is_empty() || url.is_empty() {
        return Err(AppError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "name and url are required",
        ));
    }
    let monitor_type = body
        .get("monitor_type")
        .and_then(Value::as_str)
        .unwrap_or("http_get")
        .to_string();
    if !TYPES.contains(&monitor_type.as_str()) {
        return Err(AppError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid monitor_type",
        ));
    }
    let expected_status_codes = body
        .get("expected_status_codes")
        .and_then(Value::as_str)
        .unwrap_or("2xx")
        .to_string();
    parse_status_expression(&expected_status_codes)
        .map_err(|err| AppError::new(StatusCode::UNPROCESSABLE_ENTITY, err))?;
    let expected_body_type = body
        .get("expected_body_type")
        .and_then(Value::as_str)
        .map(str::to_string);
    if let Some(kind) = &expected_body_type {
        if !BODY_TYPES.contains(&kind.as_str()) {
            return Err(AppError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid expected_body_type",
            ));
        }
    }
    let interval = body
        .get("check_interval_seconds")
        .and_then(Value::as_i64)
        .unwrap_or(60);
    if interval < 1 {
        return Err(AppError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "check_interval_seconds must be at least 1",
        ));
    }
    Ok(Fields {
        name,
        url,
        monitor_type,
        group_id: json_i64(body, "group_id"),
        is_enabled: body
            .get("is_enabled")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        check_interval_seconds: interval,
        timeout_seconds: body
            .get("timeout_seconds")
            .and_then(Value::as_i64)
            .unwrap_or(10)
            .max(1),
        retry_count: body
            .get("retry_count")
            .and_then(Value::as_i64)
            .unwrap_or(3)
            .max(1),
        retry_interval_seconds: body
            .get("retry_interval_seconds")
            .and_then(Value::as_i64)
            .unwrap_or(20),
        alert_cooldown_seconds: body
            .get("alert_cooldown_seconds")
            .and_then(Value::as_i64)
            .unwrap_or(1800),
        request_headers: body
            .get("request_headers")
            .and_then(Value::as_str)
            .map(str::to_string),
        request_body: body
            .get("request_body")
            .and_then(Value::as_str)
            .map(str::to_string),
        expected_status_codes,
        expected_body_type,
        expected_body_value: body
            .get("expected_body_value")
            .and_then(Value::as_str)
            .map(str::to_string),
        response_time_warning_ms: body
            .get("response_time_warning_ms")
            .and_then(Value::as_i64)
            .unwrap_or(2000),
        ssl_check_enabled: body
            .get("ssl_check_enabled")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        ssl_expiry_warning_days: body
            .get("ssl_expiry_warning_days")
            .and_then(Value::as_i64)
            .unwrap_or(30),
        alerts_enabled: body
            .get("alerts_enabled")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        heartbeat_grace_seconds: body
            .get("heartbeat_grace_seconds")
            .and_then(Value::as_i64)
            .unwrap_or(60),
    })
}

fn json_i64(body: &Value, key: &str) -> Option<i64> {
    body.get(key).and_then(Value::as_i64)
}

async fn apply_changes(
    state: &AppState,
    id: i64,
    body: &Map<String, Value>,
    patch: bool,
) -> AppResult<()> {
    let allowed: &[&str] = if patch {
        &[
            "is_enabled",
            "alerts_enabled",
            "ssl_check_enabled",
            "group_id",
        ]
    } else {
        &[
            "name",
            "url",
            "monitor_type",
            "group_id",
            "is_enabled",
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
        ]
    };
    for key in allowed {
        if !body.contains_key(*key) {
            continue;
        }
        let value = &body[*key];
        match *key {
            "name"
            | "url"
            | "request_headers"
            | "request_body"
            | "expected_status_codes"
            | "expected_body_type"
            | "expected_body_value" => {
                let text = value.as_str().map(str::to_string);
                if matches!(*key, "name" | "url" | "expected_status_codes")
                    && text.as_deref().unwrap_or("").is_empty()
                {
                    return Err(AppError::new(
                        StatusCode::UNPROCESSABLE_ENTITY,
                        format!("{key} is required"),
                    ));
                }
                if *key == "expected_status_codes" {
                    parse_status_expression(text.as_deref().unwrap_or(""))
                        .map_err(|err| AppError::new(StatusCode::UNPROCESSABLE_ENTITY, err))?;
                }
                if *key == "expected_body_type" {
                    if let Some(kind) = &text {
                        if !BODY_TYPES.contains(&kind.as_str()) {
                            return Err(AppError::new(
                                StatusCode::UNPROCESSABLE_ENTITY,
                                "invalid expected_body_type",
                            ));
                        }
                    }
                }
                let sql = format!("UPDATE monitors SET {key} = ? WHERE id = ?");
                sqlx::query(sqlx::AssertSqlSafe(sql))
                    .bind(text)
                    .bind(id)
                    .execute(&state.db)
                    .await?;
            }
            "monitor_type" => {
                let kind = value.as_str().unwrap_or("");
                if !TYPES.contains(&kind) {
                    return Err(AppError::new(
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "invalid monitor_type",
                    ));
                }
                sqlx::query("UPDATE monitors SET monitor_type = ? WHERE id = ?")
                    .bind(kind)
                    .bind(id)
                    .execute(&state.db)
                    .await?;
            }
            "group_id"
            | "check_interval_seconds"
            | "timeout_seconds"
            | "retry_count"
            | "retry_interval_seconds"
            | "alert_cooldown_seconds"
            | "response_time_warning_ms"
            | "ssl_expiry_warning_days"
            | "heartbeat_grace_seconds" => {
                let number = if value.is_null() {
                    None
                } else {
                    value.as_i64()
                };
                if *key == "check_interval_seconds" && number.unwrap_or(1) < 1 {
                    return Err(AppError::new(
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "check_interval_seconds must be at least 1",
                    ));
                }
                let sql = format!("UPDATE monitors SET {key} = ? WHERE id = ?");
                sqlx::query(sqlx::AssertSqlSafe(sql))
                    .bind(number)
                    .bind(id)
                    .execute(&state.db)
                    .await?;
            }
            "is_enabled" | "alerts_enabled" | "ssl_check_enabled" => {
                let flag = value.as_bool().ok_or_else(|| {
                    AppError::new(
                        StatusCode::UNPROCESSABLE_ENTITY,
                        format!("{key} must be a boolean"),
                    )
                })?;
                let sql = format!("UPDATE monitors SET {key} = ? WHERE id = ?");
                sqlx::query(sqlx::AssertSqlSafe(sql))
                    .bind(i64::from(flag))
                    .bind(id)
                    .execute(&state.db)
                    .await?;
            }
            _ => {}
        }
    }
    sqlx::query("UPDATE monitors SET updated_at = ? WHERE id = ?")
        .bind(format_db(now()))
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(())
}

async fn sync_job(state: &AppState, id: i64) -> AppResult<()> {
    let row: (i64, i64) =
        sqlx::query_as("SELECT is_enabled, check_interval_seconds FROM monitors WHERE id = ?")
            .bind(id)
            .fetch_one(&state.db)
            .await?;
    if row.0 != 0 {
        state.scheduler.upsert(state.clone(), id, row.1);
    } else {
        state.scheduler.pause(id);
    }
    Ok(())
}

async fn load(state: &AppState, id: i64) -> AppResult<MonitorRow> {
    sqlx::query_as::<_, MonitorRow>(sqlx::AssertSqlSafe(format!(
        "SELECT {COLS} FROM monitors WHERE id = ?"
    )))
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::new(StatusCode::NOT_FOUND, "Monitor not found"))
}

async fn load_visible(state: &AppState, user: &AuthUser, id: i64) -> AppResult<MonitorRow> {
    let row = load(state, id).await?;
    if user.is_privileged() || row.created_by == Some(user.id) {
        return Ok(row);
    }
    let assigned: Option<(i64,)> =
        sqlx::query_as("SELECT user_id FROM monitor_users WHERE monitor_id = ? AND user_id = ?")
            .bind(id)
            .bind(user.id)
            .fetch_optional(&state.db)
            .await?;
    if assigned.is_some() {
        Ok(row)
    } else {
        Err(AppError::new(StatusCode::NOT_FOUND, "Monitor not found"))
    }
}

fn monitor_json(row: MonitorRow, hide_token: bool) -> Value {
    let token = if hide_token {
        None
    } else {
        row.heartbeat_token.clone()
    };
    json!({
        "id": row.id,
        "name": row.name,
        "url": row.url,
        "monitor_type": row.monitor_type,
        "group_id": row.group_id,
        "is_enabled": row.is_enabled != 0,
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
        "heartbeat_token": token,
        "heartbeat_grace_seconds": row.heartbeat_grace_seconds,
        "current_status": row.current_status,
        "last_checked_at": row.last_checked_at.as_deref().map(stamp),
        "last_response_time_ms": row.last_response_time_ms,
        "consecutive_failures": row.consecutive_failures,
        "created_at": stamp(&row.created_at),
        "updated_at": stamp(&row.updated_at),
    })
}
