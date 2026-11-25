use sqlx::SqlitePool;

use crate::error::AppError;
use crate::timeutil::{format_db, now};

pub async fn write(
    pool: &SqlitePool,
    user_id: Option<i64>,
    action: &str,
    resource_type: Option<&str>,
    resource_id: Option<i64>,
    detail: Option<&str>,
    ip: Option<&str>,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO audit_logs (user_id, action, resource_type, resource_id, detail, ip_address, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind(action)
    .bind(resource_type)
    .bind(resource_id)
    .bind(detail)
    .bind(ip)
    .bind(format_db(now()))
    .execute(pool)
    .await?;
    Ok(())
}
