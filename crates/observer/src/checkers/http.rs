use std::time::Duration;

use reqwest::Client;
use serde_json::Value;

use super::matchers::{matches_body, matches_status_code};
use super::CheckOutcome;

pub struct HttpProbe<'a> {
    pub method: &'a str,
    pub url: &'a str,
    pub headers_json: Option<&'a str>,
    pub body: Option<&'a str>,
    pub expected_status: &'a str,
    pub body_type: Option<&'a str>,
    pub body_value: Option<&'a str>,
    pub timeout_seconds: i64,
    pub warning_ms: i64,
}

pub async fn http_check(client: &Client, probe: HttpProbe<'_>) -> CheckOutcome {
    let HttpProbe {
        method,
        url,
        headers_json,
        body,
        expected_status,
        body_type,
        body_value,
        timeout_seconds,
        warning_ms,
    } = probe;
    let started = std::time::Instant::now();
    let headers = parse_headers(headers_json);
    let timeout = Duration::from_secs(timeout_seconds.max(1) as u64);
    let mut request = client.request(
        reqwest::Method::from_bytes(method.as_bytes()).unwrap_or(reqwest::Method::GET),
        url,
    );
    request = request.timeout(timeout);
    for (key, value) in &headers {
        request = request.header(key, value);
    }
    if let Some(body) = body {
        request = request.body(body.to_string());
    }

    let response = match request.send().await {
        Ok(response) => response,
        Err(err) if err.is_timeout() => {
            return CheckOutcome::down(elapsed_ms(started), "Connection timed out");
        }
        Err(err) => {
            return CheckOutcome::down(elapsed_ms(started), truncate(&err.to_string()));
        }
    };

    let status_code = i64::from(response.status().as_u16());
    let bytes = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(err) => return CheckOutcome::down(elapsed_ms(started), truncate(&err.to_string())),
    };
    let limited = if bytes.len() > 256 * 1024 {
        &bytes[..256 * 1024]
    } else {
        &bytes
    };
    let text = String::from_utf8_lossy(limited);
    let elapsed = elapsed_ms(started);
    let status_ok = matches_status_code(status_code as i32, expected_status);
    let body_ok = matches_body(&text, body_type, body_value);
    if status_ok && body_ok {
        let status = if elapsed > warning_ms {
            "warning"
        } else {
            "up"
        };
        return CheckOutcome {
            status: status.to_string(),
            response_time_ms: Some(elapsed),
            status_code: Some(status_code),
            ..CheckOutcome::default()
        };
    }
    let mut reasons = Vec::new();
    if !status_ok {
        reasons.push(format!(
            "status {status_code} did not match '{expected_status}'"
        ));
    }
    if !body_ok {
        let kind = body_type.unwrap_or("");
        let value = body_value.unwrap_or("");
        reasons.push(format!("body did not match {kind}='{value}'"));
    }
    CheckOutcome {
        status: "down".to_string(),
        response_time_ms: Some(elapsed),
        status_code: Some(status_code),
        error_message: Some(reasons.join("; ")),
        ..CheckOutcome::default()
    }
}

fn parse_headers(raw: Option<&str>) -> Vec<(String, String)> {
    let Some(raw) = raw else {
        return Vec::new();
    };
    let Ok(Value::Object(map)) = serde_json::from_str::<Value>(raw) else {
        return Vec::new();
    };
    map.into_iter()
        .filter_map(|(key, value)| match value {
            Value::String(text) => Some((key, text)),
            Value::Number(number) => Some((key, number.to_string())),
            Value::Bool(flag) => Some((key, flag.to_string())),
            _ => None,
        })
        .collect()
}

fn elapsed_ms(started: std::time::Instant) -> i64 {
    i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX)
}

fn truncate(value: &str) -> String {
    value.chars().take(255).collect()
}

pub fn http_method(monitor_type: &str) -> &'static str {
    match monitor_type {
        "http_post" => "POST",
        "http_head" => "HEAD",
        _ => "GET",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get;
    use axum::Router;

    async fn serve() -> String {
        let app = Router::new()
            .route(
                "/ok",
                get(|| async { "pong" })
                    .post(|| async { "pong" })
                    .head(|| async { "" }),
            )
            .route("/big", get(|| async { "a".repeat(300_000) }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.ok();
        });
        format!("http://{addr}")
    }

    fn client() -> Client {
        Client::builder().build().unwrap()
    }

    #[tokio::test]
    async fn local_probes_cover_status_body_and_headers() {
        let base = serve().await;
        let client = client();
        let up = http_check(
            &client,
            HttpProbe {
                method: "GET",
                url: &format!("{base}/ok"),
                headers_json: Some(r#"{"X-Test":"1","N":2,"Flag":true,"Skip":{"a":1}}"#),
                body: None,
                expected_status: "2xx",
                body_type: Some("contains"),
                body_value: Some("pon"),
                timeout_seconds: 2,
                warning_ms: 5_000,
            },
        )
        .await;
        assert_eq!(up.status, "up");

        let warning = http_check(
            &client,
            HttpProbe {
                method: http_method("http_head"),
                url: &format!("{base}/ok"),
                headers_json: Some("not-json"),
                body: None,
                expected_status: "200|204",
                body_type: None,
                body_value: None,
                timeout_seconds: 2,
                warning_ms: -1,
            },
        )
        .await;
        assert_eq!(warning.status, "warning");

        let posted = http_check(
            &client,
            HttpProbe {
                method: http_method("http_post"),
                url: &format!("{base}/ok"),
                headers_json: None,
                body: Some("payload"),
                expected_status: "2xx",
                body_type: Some("equals"),
                body_value: Some("pong"),
                timeout_seconds: 2,
                warning_ms: 5_000,
            },
        )
        .await;
        assert_eq!(posted.status, "up");

        let mismatch = http_check(
            &client,
            HttpProbe {
                method: "NOPE",
                url: &format!("{base}/ok"),
                headers_json: None,
                body: None,
                expected_status: "404",
                body_type: Some("not_equals"),
                body_value: Some("pong"),
                timeout_seconds: 2,
                warning_ms: 5_000,
            },
        )
        .await;
        assert_eq!(mismatch.status, "down");
        assert!(mismatch.error_message.unwrap().contains("status"));

        let big = http_check(
            &client,
            HttpProbe {
                method: "GET",
                url: &format!("{base}/big"),
                headers_json: None,
                body: None,
                expected_status: "2xx",
                body_type: Some("contains"),
                body_value: Some("aaa"),
                timeout_seconds: 2,
                warning_ms: 5_000,
            },
        )
        .await;
        assert_eq!(big.status, "up");

        let refused = http_check(
            &client,
            HttpProbe {
                method: "GET",
                url: "http://127.0.0.1:1",
                headers_json: None,
                body: None,
                expected_status: "2xx",
                body_type: None,
                body_value: None,
                timeout_seconds: 1,
                warning_ms: 5_000,
            },
        )
        .await;
        assert_eq!(refused.status, "down");
    }

    #[tokio::test]
    async fn stalled_peer_times_out() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                let _ = tokio::io::AsyncReadExt::read(&mut stream, &mut [0u8; 8]).await;
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        });
        let outcome = http_check(
            &client(),
            HttpProbe {
                method: "GET",
                url: &format!("http://127.0.0.1:{port}/slow"),
                headers_json: None,
                body: None,
                expected_status: "2xx",
                body_type: None,
                body_value: None,
                timeout_seconds: 1,
                warning_ms: 5_000,
            },
        )
        .await;
        assert_eq!(outcome.status, "down");
    }
}
