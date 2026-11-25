use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use observer::checkers::http::{http_check, HttpProbe};
use observer::services::checks::{run_daily_ssl_scan, run_monitor_check};
use observer::services::notify::{notify_down, notify_ssl_warning, send_email, send_telegram};
use observer::services::retention::run_retention;
use observer::{bootstrap, router, AppState, Settings};
use serde_json::{json, Value};
use tower::ServiceExt;

struct TestApp {
    app: axum::Router,
    state: AppState,
    _dir: tempfile::TempDir,
}

async fn boot() -> TestApp {
    let dir = tempfile::tempdir().expect("tempdir");
    let settings = Settings::for_test(dir.path().join("observer.db").to_string_lossy().to_string());
    let state = bootstrap(settings).await.expect("bootstrap");
    TestApp {
        app: router(state.clone()),
        state,
        _dir: dir,
    }
}

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    let request = if let Some(body) = body {
        builder
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    } else {
        builder.body(Body::empty()).unwrap()
    };
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

async fn login(app: &axum::Router, username: &str, password: &str) -> Value {
    let (status, body) = call(
        app,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({"username": username, "password": password})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

async fn local_site() -> String {
    let app = axum::Router::new().route("/health", axum::routing::get(|| async { "pong" }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    format!("http://{addr}/health")
}

async fn wait_status(app: &axum::Router, token: &str, id: i64, wanted: &str) -> Value {
    let mut last = Value::Null;
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let (status, body) = call(
            app,
            "GET",
            &format!("/api/monitors/{id}"),
            Some(token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        last = body;
        if last["current_status"] == wanted {
            return last;
        }
    }
    panic!("monitor {id} stayed {last}");
}

#[tokio::test]
async fn routes_checks_and_notifications() {
    let test = boot().await;
    let session = login(&test.app, "admin", "admin").await;
    let token = session["access_token"].as_str().unwrap().to_string();
    let refresh = session["refresh_token"].as_str().unwrap();

    let (status, refreshed) = call(
        &test.app,
        "POST",
        "/api/auth/refresh",
        None,
        Some(json!({"refresh_token": refresh})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{refreshed}");
    let (status, _) = call(
        &test.app,
        "POST",
        "/api/auth/refresh",
        None,
        Some(json!({"refresh_token": token})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = call(
        &test.app,
        "POST",
        "/api/auth/change-password",
        Some(&token),
        Some(json!({"current_password": "wrong", "new_password": "correct-horse"})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, docs) = call(&test.app, "GET", "/api/docs", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(docs["title"], "Observer Lite");
    let (status, _) = call(&test.app, "GET", "/", None, None).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(&test.app, "GET", "/dashboard", None, None).await;
    assert_eq!(status, StatusCode::OK);
    let asset = std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../frontend/dist"))
        .ok()
        .and_then(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.file_name().to_string_lossy().to_string())
                .find(|name| {
                    name.ends_with(".html") || name.ends_with(".js") || name.ends_with(".css")
                })
        });
    if let Some(name) = asset {
        let (status, _) = call(&test.app, "GET", &format!("/{name}"), None, None).await;
        assert_eq!(status, StatusCode::OK);
    }

    let (status, _) = call(&test.app, "GET", "/api/monitors", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = call(&test.app, "GET", "/api/monitors/999999", Some(&token), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let site = local_site().await;
    let (status, group) = call(
        &test.app,
        "POST",
        "/api/groups",
        Some(&token),
        Some(json!({"name": "Edge", "description": "lab"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{group}");
    let group_id = group["id"].as_i64().unwrap();
    let (status, _) = call(
        &test.app,
        "PUT",
        &format!("/api/groups/{group_id}"),
        Some(&token),
        Some(json!({"name": "Edge 2", "description": null})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, groups) = call(&test.app, "GET", "/api/groups", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!groups.as_array().unwrap().is_empty());

    let (status, monitor) = call(
        &test.app,
        "POST",
        "/api/monitors",
        Some(&token),
        Some(json!({
            "name": "Site",
            "url": site,
            "monitor_type": "http_get",
            "group_id": group_id,
            "retry_count": 1,
            "timeout_seconds": 2,
            "expected_status_codes": "2xx",
            "expected_body_type": "contains",
            "expected_body_value": "pon",
            "request_headers": "{\"Accept\":\"text/plain\"}",
            "ssl_check_enabled": false,
            "response_time_warning_ms": 5000
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{monitor}");
    let id = monitor["id"].as_i64().unwrap();

    let (status, _) = call(
        &test.app,
        "PUT",
        &format!("/api/monitors/{id}"),
        Some(&token),
        Some(json!({"name": "Site renamed", "check_interval_seconds": 30})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(
        &test.app,
        "PATCH",
        &format!("/api/monitors/{id}"),
        Some(&token),
        Some(json!({"alerts_enabled": true, "group_id": group_id})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, listed) = call(
        &test.app,
        "GET",
        &format!("/api/monitors?status=pending&group_id={group_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed.as_array().unwrap().len(), 1);

    let (status, channel) = call(
        &test.app,
        "POST",
        "/api/channels",
        Some(&token),
        Some(json!({
            "name": "Mail",
            "channel_type": "email",
            "config": {
                "smtp_host": "127.0.0.1",
                "smtp_port": 1,
                "smtp_user": "mailer",
                "smtp_from": "mailer@example.com",
                "smtp_password_enc": "secret",
                "recipients": ["ops@example.com"],
                "use_tls": false
            }
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{channel}");
    let channel_id = channel["id"].as_i64().unwrap();
    let (status, _) = call(
        &test.app,
        "PUT",
        &format!("/api/channels/{channel_id}"),
        Some(&token),
        Some(json!({"name": "Mail 2", "config": {"smtp_password_enc": "***", "recipients": ["ops@example.com"], "smtp_host": "127.0.0.1", "smtp_port": 1, "smtp_from": "mailer@example.com", "use_tls": true}})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, fetched) = call(
        &test.app,
        "GET",
        &format!("/api/channels/{channel_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(fetched["config"].as_str().unwrap().contains("***"));
    let (status, channels) = call(&test.app, "GET", "/api/channels", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(channels.as_array().unwrap().len(), 1);
    let (status, failed) = call(
        &test.app,
        "POST",
        &format!("/api/channels/{channel_id}/test"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{failed}");

    let (status, telegram) = call(
        &test.app,
        "POST",
        "/api/channels",
        Some(&token),
        Some(json!({
            "name": "Tg",
            "channel_type": "telegram",
            "config": {"bot_token": "", "chat_id": ""}
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{telegram}");
    let telegram_id = telegram["id"].as_i64().unwrap();
    let (status, _) = call(
        &test.app,
        "POST",
        &format!("/api/channels/{telegram_id}/test"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);

    let (status, other) = call(
        &test.app,
        "POST",
        "/api/channels",
        Some(&token),
        Some(json!({"name": "Other", "channel_type": "slack", "config": {}})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{other}");
    let other_id = other["id"].as_i64().unwrap();
    let (status, _) = call(
        &test.app,
        "POST",
        &format!("/api/channels/{other_id}/test"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = call(
        &test.app,
        "DELETE",
        &format!("/api/channels/{other_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(
        &test.app,
        "DELETE",
        &format!("/api/channels/{other_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, assigned) = call(
        &test.app,
        "POST",
        &format!("/api/monitors/{id}/channels"),
        Some(&token),
        Some(json!({"channel_id": channel_id, "on_down": true, "on_recovery": true, "on_ssl_warn": true})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{assigned}");
    let (status, _) = call(
        &test.app,
        "POST",
        &format!("/api/monitors/{id}/channels"),
        Some(&token),
        Some(json!({"channel_id": channel_id})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, links) = call(
        &test.app,
        "GET",
        &format!("/api/monitors/{id}/channels"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(links.as_array().unwrap().len(), 1);

    let (status, viewer) = call(
        &test.app,
        "POST",
        "/api/users",
        Some(&token),
        Some(json!({
            "username": "viewer1",
            "email": "viewer@example.com",
            "password": "viewer-pass",
            "role": "viewer"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{viewer}");
    let viewer_id = viewer["id"].as_i64().unwrap();
    let (status, users) = call(&test.app, "GET", "/api/users", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(users.as_array().unwrap().len() >= 2);
    let (status, _) = call(
        &test.app,
        "GET",
        &format!("/api/users/{viewer_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(
        &test.app,
        "PUT",
        &format!("/api/users/{viewer_id}"),
        Some(&token),
        Some(json!({"email": "not-an-email"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _) = call(
        &test.app,
        "PUT",
        &format!("/api/users/{viewer_id}"),
        Some(&token),
        Some(json!({"role": "this-role-name-is-far-too-long"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, updated) = call(
        &test.app,
        "PUT",
        &format!("/api/users/{viewer_id}"),
        Some(&token),
        Some(json!({"email": "viewer2@example.com"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    let (status, _) = call(
        &test.app,
        "POST",
        &format!("/api/users/{viewer_id}/reset-password"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, member) = call(
        &test.app,
        "POST",
        &format!("/api/monitors/{id}/users?user_id={viewer_id}&notify=true"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{member}");
    let (status, _) = call(
        &test.app,
        "POST",
        &format!("/api/monitors/{id}/users?user_id={viewer_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, members) = call(
        &test.app,
        "GET",
        &format!("/api/monitors/{id}/users"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(members.as_array().unwrap().len(), 1);
    let (status, _) = call(
        &test.app,
        "POST",
        &format!("/api/monitors/{id}/users?user_id=999999"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = call(
        &test.app,
        "DELETE",
        &format!("/api/monitors/{id}/users/{viewer_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let viewer_login = login(&test.app, "viewer1", "password123").await;
    let viewer_token = viewer_login["access_token"].as_str().unwrap();
    let (status, _) = call(
        &test.app,
        "POST",
        "/api/monitors",
        Some(viewer_token),
        Some(json!({"name": "Nope", "url": "http://127.0.0.1"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = call(&test.app, "GET", "/api/users", Some(viewer_token), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = call(
        &test.app,
        "PUT",
        &format!("/api/users/{viewer_id}"),
        Some(&token),
        Some(json!({"role": "admin", "is_active": true})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let admin_login = login(&test.app, "viewer1", "password123").await;
    let (status, _) = call(
        &test.app,
        "GET",
        "/api/channels",
        Some(admin_login["access_token"].as_str().unwrap()),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(
        &test.app,
        "PUT",
        &format!("/api/users/{viewer_id}"),
        Some(&token),
        Some(json!({"is_active": false})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(
        &test.app,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({"username": "viewer1", "password": "password123"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let me = call(&test.app, "GET", "/api/auth/me", Some(&token), None)
        .await
        .1;
    let (status, denied) = call(
        &test.app,
        "DELETE",
        &format!("/api/users/{}", me["id"].as_i64().unwrap()),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(denied["detail"], "Cannot delete yourself");

    let (status, _) = call(
        &test.app,
        "POST",
        &format!("/api/monitors/{id}/check-now"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let up = wait_status(&test.app, &token, id, "up").await;
    assert!(up["last_response_time_ms"].as_i64().is_some());

    let (status, history) = call(
        &test.app,
        "GET",
        &format!(
            "/api/check-results?monitor_id={id}&from=2000-01-01T00:00:00Z&to=2999-01-01T00:00:00Z"
        ),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!history.as_array().unwrap().is_empty());
    let (status, _) = call(
        &test.app,
        "GET",
        "/api/check-results?limit=1001",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let (status, _) = call(
        &test.app,
        "PUT",
        &format!("/api/monitors/{id}"),
        Some(&token),
        Some(
            json!({"url": "http://127.0.0.1:1", "timeout_seconds": 1, "expected_body_type": null}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(
        &test.app,
        "POST",
        &format!("/api/monitors/{id}/check-now"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    wait_status(&test.app, &token, id, "down").await;
    let (status, incident) = call(
        &test.app,
        "GET",
        &format!("/api/incidents?monitor_id={id}&open_only=true"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{incident}");
    let incident_id = incident[0]["id"].as_i64().unwrap();
    let (status, one) = call(
        &test.app,
        "GET",
        &format!("/api/incidents/{incident_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{one}");

    let (status, _) = call(
        &test.app,
        "PUT",
        &format!("/api/monitors/{id}"),
        Some(&token),
        Some(json!({"url": site, "expected_body_type": "contains", "expected_body_value": "pon"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(
        &test.app,
        "POST",
        &format!("/api/monitors/{id}/check-now"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    wait_status(&test.app, &token, id, "up").await;

    let (status, window) = call(
        &test.app,
        "POST",
        "/api/maintenance",
        Some(&token),
        Some(json!({
            "name": "Now",
            "starts_at": "2020-01-01T00:00:00Z",
            "ends_at": "2999-01-01T00:00:00Z",
            "repeat_cron": null,
            "monitor_ids": [id]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{window}");
    let window_id = window["id"].as_i64().unwrap();
    let (status, _) = call(
        &test.app,
        "PUT",
        &format!("/api/maintenance/{window_id}"),
        Some(&token),
        Some(json!({
            "name": "Now 2",
            "suppress_alerts": false,
            "repeat_cron": "",
            "monitor_ids": [id]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, windows) = call(&test.app, "GET", "/api/maintenance", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(windows.as_array().unwrap().len(), 1);
    let (status, _) = call(
        &test.app,
        "GET",
        &format!("/api/maintenance/{window_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(
        &test.app,
        "POST",
        &format!("/api/monitors/{id}/check-now"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    tokio::time::sleep(Duration::from_millis(200)).await;
    let (status, _) = call(
        &test.app,
        "DELETE",
        &format!("/api/maintenance/{window_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, beat) = call(
        &test.app,
        "POST",
        "/api/monitors",
        Some(&token),
        Some(json!({
            "name": "Pulse",
            "url": "heartbeat://pulse",
            "monitor_type": "heartbeat",
            "retry_count": 1,
            "heartbeat_grace_seconds": 0
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{beat}");
    let beat_id = beat["id"].as_i64().unwrap();
    let pulse = beat["heartbeat_token"].as_str().unwrap();
    let (status, _) = call(
        &test.app,
        "POST",
        &format!("/api/monitors/{beat_id}/check-now"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    wait_status(&test.app, &token, beat_id, "down").await;
    let (status, _) = call(
        &test.app,
        "POST",
        &format!("/api/heartbeat/{pulse}"),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, after) = call(
        &test.app,
        "GET",
        &format!("/api/monitors/{beat_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(after["current_status"], "up");

    let (status, ping) = call(
        &test.app,
        "POST",
        "/api/monitors",
        Some(&token),
        Some(json!({"name": "Loop", "url": "127.0.0.1", "monitor_type": "ping", "timeout_seconds": 2, "retry_count": 1})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{ping}");
    run_monitor_check(&test.state, ping["id"].as_i64().unwrap()).await;

    let (status, secure) = call(
        &test.app,
        "POST",
        "/api/monitors",
        Some(&token),
        Some(json!({
            "name": "Cert",
            "url": "https://127.0.0.1:1",
            "ssl_check_enabled": true,
            "ssl_expiry_warning_days": 3650,
            "timeout_seconds": 1,
            "retry_count": 1
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{secure}");
    run_monitor_check(&test.state, secure["id"].as_i64().unwrap()).await;
    run_daily_ssl_scan(&test.state).await;

    sqlx::query("INSERT INTO check_results (monitor_id, checked_at, status) VALUES (?, '2000-01-01 00:00:00.000000', 'up')")
        .bind(id)
        .execute(&test.state.db)
        .await
        .unwrap();
    let deleted = run_retention(&test.state).await.unwrap();
    assert!(deleted >= 1);
    assert_eq!(run_retention(&test.state).await.unwrap(), 0);

    assert!(!send_email(&test.state, "nope", "s", "h").await);
    assert!(!send_email(&test.state, r#"{"smtp_host":"","recipients":[]}"#, "s", "h").await);
    assert!(
        !send_email(
            &test.state,
            r#"{"smtp_host":"127.0.0.1","recipients":["ops@example.com"]}"#,
            "s",
            "<p>x</p>"
        )
        .await
    );
    assert!(
        !send_telegram(&test.state, "nope", "hi").await
            && !send_telegram(&test.state, r#"{"bot_token":"","chat_id":""}"#, "hi").await
    );
    notify_down(&test.state, id, "Site", &site, "now", None).await;
    notify_ssl_warning(&test.state, id, "Site", &site, 3).await;

    let (status, _) = call(
        &test.app,
        "PUT",
        "/api/settings",
        Some(&token),
        Some(json!({
            "smtp_host": "127.0.0.1",
            "smtp_port": 1,
            "smtp_from": "mailer@example.com",
            "smtp_user": "mailer",
            "alerts_enabled": true,
            "empty": null
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(
        &test.app,
        "PUT",
        "/api/settings",
        Some(&token),
        Some(
            json!({"this-key-is-intentionally-longer-than-sixty-four-characters-so-it-fails": "x"}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _) = call(
        &test.app,
        "POST",
        "/api/settings/test-smtp",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);

    let (status, imported) = call(
        &test.app,
        "POST",
        "/api/import/monitors",
        Some(&token),
        Some(json!({"monitors": [{
            "name": "Imported",
            "url": "https://example.com",
            "monitor_type": "http_head",
            "check_interval_seconds": 120,
            "expected_body_type": "equals",
            "expected_body_value": "ok",
            "request_headers": "{}",
            "request_body": "",
            "ssl_check_enabled": false,
            "alerts_enabled": false
        }, {
            "name": "Beat",
            "url": "heartbeat://imported",
            "monitor_type": "heartbeat"
        }]})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{imported}");
    assert_eq!(imported["imported"], 2);
    let (status, exported) =
        call(&test.app, "GET", "/api/export/monitors", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(exported["monitors"].as_array().unwrap().len() >= 2);

    let (status, keys) = call(&test.app, "GET", "/api/api-keys", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(keys.as_array().unwrap().is_empty());
    let (status, key) = call(
        &test.app,
        "POST",
        "/api/api-keys",
        Some(&token),
        Some(json!({"name": "ci", "expires_at": "2999-01-01T00:00:00Z"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{key}");
    let key_id = key["id"].as_i64().unwrap();
    let (status, _) = call(
        &test.app,
        "DELETE",
        &format!("/api/api-keys/{key_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(
        &test.app,
        "DELETE",
        &format!("/api/api-keys/{key_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let admin_id = me["id"].as_i64().unwrap();
    let (status, audit) = call(
        &test.app,
        "GET",
        &format!("/api/audit-log?action=monitor.create&user_id={admin_id}&limit=10"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!audit.as_array().unwrap().is_empty());

    let (status, _) = call(
        &test.app,
        "POST",
        "/api/monitors/bulk",
        Some(&token),
        Some(json!({"action": "disable", "ids": [id, 999999]})),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(
        &test.app,
        "POST",
        "/api/monitors/bulk",
        Some(&token),
        Some(json!({"action": "enable", "ids": [id]})),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(
        &test.app,
        "POST",
        "/api/monitors/bulk",
        Some(&token),
        Some(json!({"action": "unknown", "ids": [id]})),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(
        &test.app,
        "DELETE",
        &format!("/api/monitors/{id}/channels/{channel_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(
        &test.app,
        "DELETE",
        &format!("/api/groups/{group_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(
        &test.app,
        "DELETE",
        "/api/groups/999999",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(
        &test.app,
        "POST",
        "/api/monitors/bulk",
        Some(&token),
        Some(json!({"action": "delete", "ids": [id, beat_id]})),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(
        &test.app,
        "DELETE",
        &format!("/api/users/{viewer_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(
        &test.app,
        "DELETE",
        &format!("/api/channels/{channel_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(
        &test.app,
        "DELETE",
        &format!("/api/channels/{telegram_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let probe = http_check(
        &test.state.http,
        HttpProbe {
            method: "GET",
            url: &site,
            headers_json: None,
            body: None,
            expected_status: "2xx",
            body_type: Some("contains"),
            body_value: Some("pon"),
            timeout_seconds: 2,
            warning_ms: 5_000,
        },
    )
    .await;
    assert_eq!(probe.status, "up");

    let request = Request::builder()
        .uri(format!("/api/sse/dashboard?token={token}"))
        .body(Body::empty())
        .unwrap();
    let response = test.app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let mut body = response.into_body();
    let reading =
        tokio::spawn(
            async move { tokio::time::timeout(Duration::from_secs(2), body.frame()).await },
        );
    for _ in 0..30 {
        test.state
            .sse
            .publish("monitor.check_result", &json!({"monitor_id": 1}));
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let frame = reading.await.unwrap().expect("sse timed out");
    let frame = frame.unwrap().unwrap();
    let bytes = frame.into_data().unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("monitor.check_result"));

    test.state.scheduler.shutdown();
}
