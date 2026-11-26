use std::sync::Mutex;

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::config::Settings;
use crate::crypto::{hash_password, verify_password};
use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(rename = "type")]
    pub typ: String,
    pub exp: usize,
}

pub fn create_access_token(
    settings: &Settings,
    user_id: i64,
    role: &str,
) -> Result<String, AppError> {
    let exp = (chrono::Utc::now() + chrono::Duration::minutes(settings.access_token_expire_minutes))
        .timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        role: Some(role.to_string()),
        typ: "access".to_string(),
        exp,
    };
    encode_claims(settings, &claims)
}

pub fn create_refresh_token(settings: &Settings, user_id: i64) -> Result<String, AppError> {
    let exp = (chrono::Utc::now() + chrono::Duration::days(settings.refresh_token_expire_days))
        .timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        role: None,
        typ: "refresh".to_string(),
        exp,
    };
    encode_claims(settings, &claims)
}

fn encode_claims(settings: &Settings, claims: &Claims) -> Result<String, AppError> {
    encode(
        &Header::new(Algorithm::HS256),
        claims,
        &EncodingKey::from_secret(settings.secret_key.as_bytes()),
    )
    .map_err(|err| {
        tracing::error!(error = %err, "jwt encode failed");
        AppError::new(axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Token error")
    })
}

pub fn decode_token(settings: &Settings, token: &str) -> Result<Claims, AppError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.leeway = 0;
    validation.validate_exp = true;
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(settings.secret_key.as_bytes()),
        &validation,
    )
    .map(|data| data.claims)
    .map_err(|_| AppError::new(axum::http::StatusCode::UNAUTHORIZED, "Invalid token"))
}

pub async fn hash_password_async(password: String) -> Result<String, AppError> {
    tokio::task::spawn_blocking(move || hash_password(&password))
        .await
        .map_err(|_| {
            AppError::new(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Password hashing failed",
            )
        })?
        .map_err(|_| {
            AppError::new(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Password hashing failed",
            )
        })
}

pub async fn verify_password_async(password: String, password_hash: String) -> bool {
    tokio::task::spawn_blocking(move || verify_password(&password, &password_hash))
        .await
        .unwrap_or(false)
}

/// Five login attempts per minute, per IP, as documented.
#[derive(Default)]
pub struct LoginLimiter {
    hits: Mutex<std::collections::HashMap<String, Vec<std::time::Instant>>>,
}

impl LoginLimiter {
    pub fn allow(&self, key: &str, limit: u32) -> bool {
        if limit == 0 {
            return true;
        }
        let mut hits = self.hits.lock().expect("login limiter");
        let now = std::time::Instant::now();
        let window = std::time::Duration::from_secs(60);
        let entry = hits.entry(key.to_string()).or_default();
        entry.retain(|instant| now.duration_since(*instant) < window);
        if entry.len() as u32 >= limit {
            return false;
        }
        entry.push(now);
        if hits.len() > 10_000 {
            hits.retain(|_, values| !values.is_empty());
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> Settings {
        let mut settings = Settings::from_env();
        settings.secret_key = "unit-test-secret".into();
        settings.access_token_expire_minutes = 15;
        settings.refresh_token_expire_days = 30;
        settings
    }

    #[test]
    fn access_and_refresh_roundtrip() {
        let settings = settings();
        let access = create_access_token(&settings, 7, "admin").unwrap();
        let claims = decode_token(&settings, &access).unwrap();
        assert_eq!(claims.sub, "7");
        assert_eq!(claims.typ, "access");
        assert_eq!(claims.role.as_deref(), Some("admin"));

        let refresh = create_refresh_token(&settings, 7).unwrap();
        let claims = decode_token(&settings, &refresh).unwrap();
        assert_eq!(claims.typ, "refresh");
        assert!(decode_token(&settings, "nope").is_err());
    }

    #[test]
    fn limiter_blocks_the_sixth_attempt() {
        let limiter = LoginLimiter::default();
        for _ in 0..5 {
            assert!(limiter.allow("1.1.1.1", 5));
        }
        assert!(!limiter.allow("1.1.1.1", 5));
        assert!(limiter.allow("2.2.2.2", 5));
        assert!(limiter.allow("1.1.1.1", 0));
    }
}
