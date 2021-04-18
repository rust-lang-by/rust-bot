use sqlx::types::chrono::NaiveDateTime;
use sqlx::{Error, PgPool, Pool, Postgres};
use std::env;

pub async fn establish_connection() -> Result<Pool<Postgres>, Error> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgPool::connect(&database_url).await
}

pub async fn lead_earliest_mention_time(pool: &PgPool) -> Result<NaiveDateTime, Error> {
    sqlx::query_as("SELECT updated_at FROM mentions ORDER BY updated_at DESC LIMIT 1")
        .fetch_one(pool)
        .await
        .map(|row: (NaiveDateTime,)| row.0)
}

pub async fn insert_mention(pool: &PgPool, user_id: i64) -> Result<i64, Error> {
    sqlx::query_as("INSERT INTO mentions(user_id) VALUES ($1) RETURNING id")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map(|row: (i64,)| row.0)
}
