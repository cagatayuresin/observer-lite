use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

use crate::audit;
use crate::error::{AppError, AppResult};
use crate::extract::{AdminUser, ClientIp};
use crate::routes::users::stamp;
use crate::services::notify::{
    mask_config, protect_config, restore_masked_secrets, send_email, send_telegram,
};
use crate::state::AppState;
use crate::timeutil::{format_db, now};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/channels", get(list).post(create))
        .route(
            "/api/channels/{channel_id}",
            get(get_one).put(update).delete(remove),
        )
        .route("/api/channels/{channel_id}/test", post(test_channel))
}

async fn list(State(state): State<AppState>, _user: AdminUser) -> AppResult<Json<Vec<Value>>> {
    let rows = fetch_all(&state).await?;
    Ok(Json(rows.into_iter().map(channel_json).collect()))
}

async fn create(
    State(state): State<AppState>,
    user: AdminUser,
    ClientIp(ip): ClientIp,
    Json(body): Json<Value>,
) -> AppResult<(StatusCode, Json<Value>)> {
    let name = required_str(&body, "name")?;
    let channel_type = required_str(&body, "channel_type")?;
    let mut config = body
        .get("config")
        .cloned()
        .unwrap_or(Value::Object(Default::default()));
    protect_config(&state.settings.secret_key, &mut config);
    let enabled = body
        .get("is_enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let stamp_now = format_db(now());
    let config_text = serde_json::to_string(&config).unwrap_or_else(|_| "{}".into());
    let id = sqlx::query(
        "INSERT INTO notification_channels (name, channel_type, config, is_enabled, created_by, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&name)
    .bind(&channel_type)
    .bind(&config_text)
    .bind(i64::from(enabled))
    .bind(user.id)
    .bind(&stamp_now)
    .bind(&stamp_now)
    .execute(&state.db)
    .await?
    .last_insert_rowid();
    audit::write(
        &state.db,
        Some(user.id),
        "channel.create",
        Some("channel"),
        Some(id),
        Some(&name),
        ip.as_deref(),
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(channel_json((
            id,
            name,
            channel_type,
            config_text,
            i64::from(enabled),
            stamp_now,
        ))),
    ))
}

async fn get_one(
    State(state): State<AppState>,
    _user: AdminUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    Ok(Json(channel_json(fetch(&state, id).await?)))
}

async fn update(
    State(state): State<AppState>,
    user: AdminUser,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
    Json(body): Json<Value>,
) -> AppResult<Json<Value>> {
    let current = fetch(&state, id).await?;
    let name = body
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(&current.1);
    let mut config_text = current.3.clone();
    if let Some(config) = body.get("config") {
        let mut config = config.clone();
        restore_masked_secrets(&current.3, &mut config);
        protect_config(&state.settings.secret_key, &mut config);
        config_text = serde_json::to_string(&config).unwrap_or(config_text);
    }
    let enabled = body
        .get("is_enabled")
        .and_then(Value::as_bool)
        .map(i64::from)
        .unwrap_or(current.4);
    let stamp_now = format_db(now());
    sqlx::query("UPDATE notification_channels SET name = ?, config = ?, is_enabled = ?, updated_at = ? WHERE id = ?")
        .bind(name)
        .bind(&config_text)
        .bind(enabled)
        .bind(&stamp_now)
        .bind(id)
        .execute(&state.db)
        .await?;
    audit::write(
        &state.db,
        Some(user.id),
        "channel.update",
        Some("channel"),
        Some(id),
        None,
        ip.as_deref(),
    )
    .await?;
    Ok(Json(channel_json((
        id,
        name.to_string(),
        current.2,
        config_text,
        enabled,
        current.5,
    ))))
}

async fn remove(
    State(state): State<AppState>,
    user: AdminUser,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    let changed = sqlx::query("DELETE FROM notification_channels WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?
        .rows_affected();
    if changed == 0 {
        return Err(AppError::new(StatusCode::NOT_FOUND, "Channel not found"));
    }
    audit::write(
        &state.db,
        Some(user.id),
        "channel.delete",
        Some("channel"),
        Some(id),
        None,
        ip.as_deref(),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn test_channel(
    State(state): State<AppState>,
    _user: AdminUser,
    Path(id): Path<i64>,
) -> AppResult<Json<Value>> {
    let row = fetch(&state, id).await?;
    let ok = match row.2.as_str() {
        "email" => {
            send_email(
                &state,
                &row.3,
                "Observer-Lite Test",
                "<p>Test notification from Observer-Lite.</p>",
            )
            .await
        }
        "telegram" => send_telegram(&state, &row.3, "🟢 Observer-Lite test notification.").await,
        _ => {
            return Err(AppError::new(
                StatusCode::BAD_REQUEST,
                "Unknown channel type",
            ))
        }
    };
    if !ok {
        return Err(AppError::new(
            StatusCode::BAD_GATEWAY,
            "Test notification failed — check channel configuration",
        ));
    }
    Ok(Json(
        json!({"message": "Test notification sent successfully"}),
    ))
}

type ChannelRow = (i64, String, String, String, i64, String);

async fn fetch(state: &AppState, id: i64) -> AppResult<ChannelRow> {
    sqlx::query_as(
        "SELECT id, name, channel_type, config, is_enabled, created_at FROM notification_channels WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::new(StatusCode::NOT_FOUND, "Channel not found"))
}

async fn fetch_all(state: &AppState) -> AppResult<Vec<ChannelRow>> {
    Ok(sqlx::query_as(
        "SELECT id, name, channel_type, config, is_enabled, created_at FROM notification_channels ORDER BY name",
    )
    .fetch_all(&state.db)
    .await?)
}

fn channel_json(row: ChannelRow) -> Value {
    json!({
        "id": row.0,
        "name": row.1,
        "channel_type": row.2,
        "config": mask_config(&row.3),
        "is_enabled": row.4 != 0,
        "created_at": stamp(&row.5),
    })
}

fn required_str(body: &Value, key: &str) -> AppResult<String> {
    body.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            AppError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                format!("{key} is required"),
            )
        })
}
