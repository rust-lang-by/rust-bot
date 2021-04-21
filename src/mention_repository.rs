use sqlx::types::chrono::NaiveDateTime;
use sqlx::{Error, PgPool, Pool, Postgres};
use std::env;

pub async fn lead_earliest_mention_time(pool: &PgPool) -> Result<NaiveDateTime, Error> {
    sqlx::query_as("SELECT updated_at FROM mentions ORDER BY updated_at DESC LIMIT 1")
        .fetch_one(pool)
        .await
        .map(|row: (NaiveDateTime,)| row.0)
}

pub async fn insert_mention(pool: &PgPool, user_id: i64) -> Result<(i64,), Error> {
    sqlx::query_as("INSERT INTO mentions(user_id) VALUES ($1)  ON CONFLICT (user_id) DO UPDATE SET updated_at = current_timestamp RETURNING user_id")
        .bind(user_id)
        .fetch_one(pool)
        .await
}
