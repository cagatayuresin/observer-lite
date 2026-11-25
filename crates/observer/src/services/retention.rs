use crate::error::AppError;
use crate::state::AppState;
use crate::timeutil::{format_db, now};

const BATCH: i64 = 10_000;

pub async fn run_retention(state: &AppState) -> Result<u64, AppError> {
    let days = retention_days(&state.db).await?;
    let cutoff = format_db(now() - chrono::Duration::days(days));
    let mut deleted = 0u64;
    loop {
        let ids: Vec<(i64,)> =
            sqlx::query_as("SELECT id FROM check_results WHERE checked_at < ? ORDER BY id LIMIT ?")
                .bind(&cutoff)
                .bind(BATCH)
                .fetch_all(&state.db)
                .await?;
        if ids.is_empty() {
            break;
        }
        let list = ids
            .iter()
            .map(|(id,)| id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!("DELETE FROM check_results WHERE id IN ({list})");
        let result = sqlx::query(sqlx::AssertSqlSafe(sql))
            .execute(&state.db)
            .await?;
        deleted += result.rows_affected();
        if ids.len() < BATCH as usize {
            break;
        }
    }
    if deleted > 0 {
        tracing::info!(deleted, days, "retention cleanup removed old check results");
    }
    Ok(deleted)
}

async fn retention_days(pool: &sqlx::SqlitePool) -> Result<i64, AppError> {
    let value: Option<(String,)> =
        sqlx::query_as("SELECT value FROM app_settings WHERE key = 'data_retention_days'")
            .fetch_optional(pool)
            .await?;
    let days = value
        .and_then(|(raw,)| raw.parse::<i64>().ok())
        .unwrap_or(90)
        .clamp(1, 3650);
    Ok(days)
}
