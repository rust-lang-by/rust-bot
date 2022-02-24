use sqlx::postgres::PgQueryResult;
use sqlx::types::chrono::NaiveDateTime;
use sqlx::PgPool;

pub async fn lead_earliest_mention_time(pool: &PgPool) -> NaiveDateTime {
    sqlx::query_as("SELECT updated_at FROM mentions ORDER BY updated_at DESC LIMIT 1")
        .fetch_one(pool)
        .await
        .map(|row: (NaiveDateTime,)| row.0)
        .expect("Can't pool latest mention time")
}

pub async fn insert_mention(pool: &PgPool, user_id: i64, username: &String) -> PgQueryResult {
    sqlx::query("INSERT INTO mentions(user_id, username) VALUES ($1, $2)  ON CONFLICT (user_id) DO UPDATE SET updated_at = current_timestamp, counter = mentions.counter + 1")
        .bind(user_id)
        .bind(username)
        .execute(pool)
        .await
        .expect("Can't insert mention")
}
