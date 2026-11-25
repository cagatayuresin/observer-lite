pub mod http;
pub mod matchers;
pub mod ping;
pub mod ssl;

use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct CheckOutcome {
    pub status: String,
    pub response_time_ms: Option<i64>,
    pub status_code: Option<i64>,
    pub is_ssl_valid: Option<bool>,
    pub ssl_expiry_days: Option<i64>,
    pub error_message: Option<String>,
    pub checked_at: DateTime<Utc>,
}

impl Default for CheckOutcome {
    fn default() -> Self {
        Self {
            status: "down".to_string(),
            response_time_ms: None,
            status_code: None,
            is_ssl_valid: None,
            ssl_expiry_days: None,
            error_message: None,
            checked_at: Utc::now(),
        }
    }
}

impl CheckOutcome {
    pub fn down(response_time_ms: i64, message: impl Into<String>) -> Self {
        Self {
            status: "down".to_string(),
            response_time_ms: Some(response_time_ms),
            error_message: Some(message.into()),
            ..Self::default()
        }
    }
}
