use serde_json::{json, Value};

use crate::crypto::decrypt_or_plain;
use crate::db::flag;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Clone)]
struct Delivery {
    channel_type: String,
    config: String,
}

pub fn format_duration(seconds: Option<i64>) -> String {
    let Some(seconds) = seconds.filter(|value| *value > 0) else {
        return "unknown duration".into();
    };
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    let minutes = (seconds % 3_600) / 60;
    let secs = seconds % 60;
    if days > 0 {
        format!("{days}d {hours}h {minutes}m")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else if minutes > 0 {
        format!("{minutes}m {secs}s")
    } else {
        format!("{secs}s")
    }
}

pub async fn notify_down(
    state: &AppState,
    monitor_id: i64,
    name: &str,
    url: &str,
    started_at: &str,
    root_cause: Option<&str>,
) {
    let reason = root_cause.unwrap_or("Unknown");
    let subject = format!("[DOWN] {name} is DOWN");
    let html = format!(
        "<h2>Monitor Down: {name}</h2><p><b>URL:</b> {url}</p><p><b>Time:</b> {started_at} UTC</p><p><b>Reason:</b> {reason}</p>"
    );
    let text =
        format!("🔴 <b>{name}</b> is DOWN\nURL: {url}\nReason: {reason}\nTime: {started_at} UTC");
    dispatch(state, monitor_id, "on_down", &subject, &html, &text).await;
}

pub async fn notify_recovery(
    state: &AppState,
    monitor_id: i64,
    name: &str,
    url: &str,
    resolved_at: &str,
    duration_seconds: Option<i64>,
) {
    let duration = format_duration(duration_seconds);
    let subject = format!("[UP] {name} recovered");
    let html = format!(
        "<h2>Monitor Recovered: {name}</h2><p><b>URL:</b> {url}</p><p><b>Recovered at:</b> {resolved_at} UTC</p><p><b>Downtime:</b> {duration}</p>"
    );
    let text = format!("🟢 <b>{name}</b> is UP again\nURL: {url}\nDowntime: {duration}\nRecovered: {resolved_at} UTC");
    dispatch(state, monitor_id, "on_recovery", &subject, &html, &text).await;
}

pub async fn notify_ssl_warning(
    state: &AppState,
    monitor_id: i64,
    name: &str,
    url: &str,
    days: i64,
) {
    let subject = format!("[SSL] {name} certificate expires in {days} days");
    let html = format!(
        "<h2>SSL Warning: {name}</h2><p>Certificate expires in <b>{days} days</b>.</p><p><b>URL:</b> {url}</p>"
    );
    let text =
        format!("⚠️ <b>SSL Warning</b>: {name}\nCertificate expires in {days} days\nURL: {url}");
    dispatch(state, monitor_id, "on_ssl_warn", &subject, &html, &text).await;
}

async fn dispatch(
    state: &AppState,
    monitor_id: i64,
    flag_name: &str,
    subject: &str,
    html: &str,
    text: &str,
) {
    let deliveries = match load_deliveries(&state.db, monitor_id, flag_name).await {
        Ok(rows) => rows,
        Err(err) => {
            tracing::error!(error = %err.detail, "notification lookup failed");
            return;
        }
    };
    for delivery in deliveries {
        let result = match delivery.channel_type.as_str() {
            "email" => send_email(state, &delivery.config, subject, html).await,
            "telegram" => send_telegram(state, &delivery.config, text).await,
            _ => false,
        };
        if !result {
            tracing::warn!(channel = %delivery.channel_type, "notification was not delivered");
        }
    }
}

async fn load_deliveries(
    pool: &sqlx::SqlitePool,
    monitor_id: i64,
    flag_name: &str,
) -> Result<Vec<Delivery>, AppError> {
    let rows = sqlx::query_as::<_, ChannelLink>(
        "SELECT c.channel_type, c.config, m.on_down, m.on_recovery, m.on_ssl_warn
         FROM monitor_notification_channels m
         JOIN notification_channels c ON c.id = m.channel_id
         WHERE m.monitor_id = ? AND c.is_enabled = 1",
    )
    .bind(monitor_id)
    .fetch_all(pool)
    .await?;

    if !rows.is_empty() {
        return Ok(rows
            .into_iter()
            .filter(|row| match flag_name {
                "on_down" => flag(row.on_down),
                "on_recovery" => flag(row.on_recovery),
                _ => flag(row.on_ssl_warn),
            })
            .map(|row| Delivery {
                channel_type: row.channel_type,
                config: row.config,
            })
            .collect());
    }

    let all = sqlx::query_as::<_, (String, String)>(
        "SELECT channel_type, config FROM notification_channels WHERE is_enabled = 1 ORDER BY id",
    )
    .fetch_all(pool)
    .await?;
    Ok(all
        .into_iter()
        .map(|(channel_type, config)| Delivery {
            channel_type,
            config,
        })
        .collect())
}

#[derive(sqlx::FromRow)]
struct ChannelLink {
    channel_type: String,
    config: String,
    on_down: i64,
    on_recovery: i64,
    on_ssl_warn: i64,
}

pub async fn send_email(state: &AppState, config: &str, subject: &str, html: &str) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(config) else {
        return false;
    };
    let host = text(&value, "smtp_host");
    let recipients = string_list(&value, "recipients");
    if host.is_empty() || recipients.is_empty() {
        return false;
    }
    let port = value
        .get("smtp_port")
        .and_then(Value::as_i64)
        .unwrap_or(587) as u16;
    let user = text(&value, "smtp_user");
    let from = {
        let configured = text(&value, "smtp_from");
        if configured.is_empty() {
            user.clone()
        } else {
            configured
        }
    };
    if from.is_empty() {
        return false;
    }
    let password = decrypt_or_plain(
        &state.settings.secret_key,
        &text(&value, "smtp_password_enc"),
    );
    let use_tls = value
        .get("use_tls")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    match deliver_smtp(SmtpMail {
        host: &host,
        port,
        user: &user,
        password: &password,
        from: &from,
        recipients: &recipients,
        subject,
        html,
        use_tls,
    })
    .await
    {
        Ok(()) => true,
        Err(err) => {
            tracing::warn!(error = %err, "smtp send failed");
            false
        }
    }
}

struct SmtpMail<'a> {
    host: &'a str,
    port: u16,
    user: &'a str,
    password: &'a str,
    from: &'a str,
    recipients: &'a [String],
    subject: &'a str,
    html: &'a str,
    use_tls: bool,
}

async fn deliver_smtp(mail: SmtpMail<'_>) -> Result<(), String> {
    let SmtpMail {
        host,
        port,
        user,
        password,
        from,
        recipients,
        subject,
        html,
        use_tls,
    } = mail;
    use lettre::message::{header::ContentType, Mailbox, Message};
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::transport::smtp::client::{Tls, TlsParameters};
    use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};

    let from_box: Mailbox = from.parse().map_err(|err| format!("from: {err}"))?;
    let mut builder = Message::builder()
        .from(from_box)
        .subject(subject)
        .header(ContentType::TEXT_HTML);
    for recipient in recipients {
        let mailbox: Mailbox = recipient.parse().map_err(|err| format!("to: {err}"))?;
        builder = builder.to(mailbox);
    }
    let email = builder
        .body(html.to_string())
        .map_err(|err| err.to_string())?;
    let tls = TlsParameters::new(host.to_string()).map_err(|err| err.to_string())?;
    let tls = match smtp_security(port, use_tls) {
        SmtpSecurity::Implicit => Tls::Wrapper(tls),
        SmtpSecurity::StartTls => Tls::Required(tls),
        SmtpSecurity::Plain => Tls::None,
    };
    let mut transport = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
        .port(port)
        .tls(tls);
    if !user.is_empty() {
        transport = transport.credentials(Credentials::new(user.to_string(), password.to_string()));
    }
    transport
        .build()
        .send(email)
        .await
        .map(|_| ())
        .map_err(|err| err.to_string())
}

pub async fn send_telegram(state: &AppState, config: &str, text_body: &str) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(config) else {
        return false;
    };
    let token = decrypt_or_plain(&state.settings.secret_key, &text(&value, "bot_token"));
    let chat_id = text(&value, "chat_id");
    if token.is_empty() || chat_id.is_empty() {
        return false;
    }
    let silent = value
        .get("disable_notification")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let url = format!("https://api.telegram.org/bot{token}/sendMessage");
    match state
        .http
        .post(url)
        .timeout(std::time::Duration::from_secs(10))
        .json(&json!({
            "chat_id": chat_id,
            "text": text_body,
            "parse_mode": "HTML",
            "disable_notification": silent,
        }))
        .send()
        .await
    {
        Ok(response) => response.status().as_u16() == 200,
        Err(err) => {
            tracing::warn!(error = %err, "telegram send failed");
            false
        }
    }
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string()
}

#[derive(Debug, PartialEq, Eq)]
enum SmtpSecurity {
    Implicit,
    StartTls,
    Plain,
}

/// Port 465 speaks TLS from the first byte. Port 587 speaks STARTTLS, including
/// when an older config stored `use_tls: false` to reach that mode.
fn smtp_security(port: u16, use_tls: bool) -> SmtpSecurity {
    if port == 465 {
        if use_tls {
            SmtpSecurity::Implicit
        } else {
            SmtpSecurity::Plain
        }
    } else if port == 587 || use_tls {
        SmtpSecurity::StartTls
    } else {
        SmtpSecurity::Plain
    }
}

fn string_list(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Encrypt plaintext secrets inside a channel config. Existing Fernet tokens and
/// the masked placeholder are left for the caller to restore.
pub fn protect_config(secret_key: &str, config: &mut Value) {
    let Some(object) = config.as_object_mut() else {
        return;
    };
    for key in ["smtp_password_enc", "bot_token"] {
        let Some(current) = object.get(key).and_then(Value::as_str).map(str::to_string) else {
            continue;
        };
        if current.is_empty() || current == "***" {
            continue;
        }
        if crate::crypto::decrypt_secret(secret_key, &current).is_ok() {
            continue;
        }
        if let Ok(token) = crate::crypto::encrypt_secret(secret_key, &current) {
            object.insert(key.to_string(), Value::String(token));
        }
    }
}

pub fn mask_config(config: &str) -> String {
    let Ok(mut value) = serde_json::from_str::<Value>(config) else {
        return config.to_string();
    };
    if let Some(object) = value.as_object_mut() {
        for key in ["smtp_password_enc", "bot_token"] {
            if object
                .get(key)
                .and_then(Value::as_str)
                .is_some_and(|v| !v.is_empty())
            {
                object.insert(key.to_string(), Value::String("***".into()));
            }
        }
    }
    serde_json::to_string(&value).unwrap_or_else(|_| config.to_string())
}

/// Keep the stored secret when the form posts the masked placeholder back.
pub fn restore_masked_secrets(previous: &str, incoming: &mut Value) {
    let Ok(old) = serde_json::from_str::<Value>(previous) else {
        return;
    };
    let Some(object) = incoming.as_object_mut() else {
        return;
    };
    for key in ["smtp_password_enc", "bot_token"] {
        let posted = object.get(key).and_then(Value::as_str).unwrap_or("");
        if posted == "***" || posted.is_empty() {
            if let Some(existing) = old.get(key) {
                if existing.as_str().is_some_and(|value| !value.is_empty()) {
                    object.insert(key.to_string(), existing.clone());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submission_port_uses_starttls() {
        assert_eq!(smtp_security(587, true), SmtpSecurity::StartTls);
        assert_eq!(smtp_security(587, false), SmtpSecurity::StartTls);
        assert_eq!(smtp_security(465, true), SmtpSecurity::Implicit);
        assert_eq!(smtp_security(465, false), SmtpSecurity::Plain);
        assert_eq!(smtp_security(25, false), SmtpSecurity::Plain);
        assert_eq!(smtp_security(2525, true), SmtpSecurity::StartTls);
    }

    #[test]
    fn duration_buckets() {
        assert_eq!(format_duration(None), "unknown duration");
        assert_eq!(format_duration(Some(0)), "unknown duration");
        assert_eq!(format_duration(Some(45)), "45s");
        assert_eq!(format_duration(Some(125)), "2m 5s");
        assert_eq!(format_duration(Some(3_700)), "1h 1m");
        assert_eq!(format_duration(Some(90_061)), "1d 1h 1m");
    }

    #[test]
    fn secrets_are_masked_and_restored() {
        let original = r#"{"smtp_password_enc":"s3cret","smtp_host":"h"}"#;
        let masked = mask_config(original);
        assert!(masked.contains("***"));
        assert!(!masked.contains("s3cret"));
        let mut incoming: Value = serde_json::from_str(&masked).unwrap();
        restore_masked_secrets(original, &mut incoming);
        assert_eq!(incoming["smtp_password_enc"], "s3cret");
    }
}
