use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use observer::{bootstrap, router, Settings};
use serde_json::{json, Value};
use tower::ServiceExt;

struct TestApp {
    app: axum::Router,
    _dir: tempfile::TempDir,
}

async fn boot() -> TestApp {
    let dir = tempfile::tempdir().expect("tempdir");
    let settings = Settings::for_test(dir.path().join("observer.db").to_string_lossy().to_string());
    let state = bootstrap(settings).await.expect("bootstrap");
    TestApp {
        app: router(state),
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
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, value)
}

async fn login(app: &axum::Router) -> String {
    let (status, body) = call(
        app,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({"username": "admin", "password": "admin"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["access_token"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn health_and_login_flow() {
    let test = boot().await;
    let (status, body) = call(&test.app, "GET", "/api/health", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");

    let (status, body) = call(
        &test.app,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({"username": "admin", "password": "nope"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["detail"], "Invalid credentials");

    let token = login(&test.app).await;
    let (status, me) = call(&test.app, "GET", "/api/auth/me", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["username"], "admin");
    assert_eq!(me["force_pw_change"], true);

    let (status, _) = call(
        &test.app,
        "POST",
        "/api/auth/change-password",
        Some(&token),
        Some(json!({"current_password": "admin", "new_password": "short"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let (status, _) = call(
        &test.app,
        "POST",
        "/api/auth/change-password",
        Some(&token),
        Some(json!({"current_password": "admin", "new_password": "correct-horse"})),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, me) = call(&test.app, "GET", "/api/auth/me", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["force_pw_change"], false);
}

#[tokio::test]
async fn monitors_incidents_and_heartbeat() {
    let test = boot().await;
    let token = login(&test.app).await;

    let (status, _) = call(
        &test.app,
        "POST",
        "/api/monitors",
        Some(&token),
        Some(json!({"name": "Bad", "url": "http://127.0.0.1:1", "expected_status_codes": "nope"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let (status, monitor) = call(
        &test.app,
        "POST",
        "/api/monitors",
        Some(&token),
        Some(json!({
            "name": "Local",
            "url": "http://127.0.0.1:1",
            "retry_count": 1,
            "timeout_seconds": 1,
            "monitor_type": "http_get"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{monitor}");
    let id = monitor["id"].as_i64().unwrap();
    assert_eq!(monitor["current_status"], "pending");

    let (status, _) = call(
        &test.app,
        "POST",
        &format!("/api/monitors/{id}/check-now"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);

    let mut incidents = Value::Array(vec![]);
    for _ in 0..40 {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (status, body) = call(
            &test.app,
            "GET",
            &format!("/api/incidents?monitor_id={id}"),
            Some(&token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        if body.as_array().is_some_and(|rows| !rows.is_empty()) {
            incidents = body;
            break;
        }
    }
    let incident_id = incidents[0]["id"].as_i64().expect("incident should open");
    let (status, _) = call(
        &test.app,
        "POST",
        &format!("/api/incidents/{incident_id}/acknowledge"),
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
        Some(json!({"name": "Pulse", "url": "heartbeat://pulse", "monitor_type": "heartbeat"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{beat}");
    let pulse = beat["heartbeat_token"].as_str().unwrap();
    let (status, pong) = call(
        &test.app,
        "GET",
        &format!("/api/heartbeat/{pulse}"),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(pong["message"], "ok");

    let (status, listed) = call(
        &test.app,
        "GET",
        "/api/monitors?search=Loc",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed.as_array().unwrap().len(), 1);

    let (status, stats) = call(
        &test.app,
        "GET",
        &format!("/api/monitors/{id}/stats?days=7"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(stats["total_checks"].as_i64().unwrap() >= 1);

    let (status, _) = call(
        &test.app,
        "POST",
        &format!("/api/monitors/{id}/pause"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(
        &test.app,
        "POST",
        &format!("/api/monitors/{id}/resume"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn admin_surfaces() {
    let test = boot().await;
    let token = login(&test.app).await;

    let (status, group) = call(
        &test.app,
        "POST",
        "/api/groups",
        Some(&token),
        Some(json!({"name": "Edge", "description": "small machines"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{group}");

    let (status, user) = call(
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
    assert_eq!(status, StatusCode::CREATED, "{user}");
    let viewer_id = user["id"].as_i64().unwrap();

    let (status, _) = call(
        &test.app,
        "POST",
        "/api/users",
        Some(&token),
        Some(
            json!({"username": "viewer1", "email": "other@example.com", "password": "viewer-pass"}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    let (status, denied) = call(
        &test.app,
        "DELETE",
        &format!("/api/users/{viewer_id}"),
        Some(&token),
        None,
    )
    .await;
    let me = call(&test.app, "GET", "/api/auth/me", Some(&token), None)
        .await
        .1;
    if me["id"] == viewer_id {
        assert_eq!(denied["detail"], "Cannot delete yourself");
    } else {
        assert_eq!(status, StatusCode::NO_CONTENT);
    }

    let (status, key) = call(
        &test.app,
        "POST",
        "/api/api-keys",
        Some(&token),
        Some(json!({"name": "ci"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{key}");
    let raw = key["raw_key"].as_str().unwrap();
    let (status, via_key) = call(&test.app, "GET", "/api/auth/me", Some(raw), None).await;
    assert_eq!(status, StatusCode::OK, "{via_key}");

    let (status, _) = call(
        &test.app,
        "PUT",
        "/api/settings",
        Some(&token),
        Some(json!({"data_retention_days": 30, "smtp_password_enc": "s3cret"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, settings) = call(&test.app, "GET", "/api/settings", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(settings["data_retention_days"], "30");
    assert_eq!(settings["smtp_password_enc"], "***");
    let (status, _) = call(
        &test.app,
        "PUT",
        "/api/settings",
        Some(&token),
        Some(json!({"smtp_password_enc": "***"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, exported) =
        call(&test.app, "GET", "/api/export/monitors", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(exported["version"], 1);

    let (status, imported) = call(
        &test.app,
        "POST",
        "/api/import/monitors",
        Some(&token),
        Some(json!({"monitors": [{"name": "Imported", "url": "https://example.com"}]})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(imported["imported"], 1);

    let (status, window) = call(
        &test.app,
        "POST",
        "/api/maintenance",
        Some(&token),
        Some(json!({
            "name": "Quiet",
            "starts_at": "2026-01-01T00:00:00Z",
            "ends_at": "2026-01-01T01:00:00Z",
            "monitor_ids": []
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{window}");

    let (status, channel) = call(
        &test.app,
        "POST",
        "/api/channels",
        Some(&token),
        Some(json!({
            "name": "Mail",
            "channel_type": "email",
            "config": {"smtp_host": "localhost", "smtp_password_enc": "plain", "recipients": ["a@b.c"]}
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{channel}");
    assert!(channel["config"].as_str().unwrap().contains("***"));
    assert!(!channel["config"].as_str().unwrap().contains("plain"));

    let (status, audit) = call(&test.app, "GET", "/api/audit-log", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(audit.as_array().unwrap().len() >= 3);

    let (status, _) = call(
        &test.app,
        "GET",
        "/api/sse/dashboard?token=nope",
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
