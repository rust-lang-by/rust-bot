use sqlx::postgres::PgQueryResult;
use sqlx::types::chrono::NaiveDateTime;
use sqlx::{Error, PgPool};

pub async fn lead_earliest_mention_time(pool: &PgPool) -> Result<NaiveDateTime, Error> {
    sqlx::query_as("SELECT updated_at FROM mentions ORDER BY updated_at DESC LIMIT 1")
        .fetch_one(pool)
        .await
        .map(|row: (NaiveDateTime,)| row.0)
}

pub async fn insert_mention(pool: &PgPool, user_id: i64) -> Result<PgQueryResult, Error> {
    sqlx::query("INSERT INTO mentions(user_id) VALUES ($1)  ON CONFLICT (user_id) DO UPDATE SET updated_at = current_timestamp")
        .bind(user_id)
        .execute(pool)
        .await
}
