use sqlx::postgres::PgQueryResult;
use sqlx::types::chrono::NaiveDateTime;
use sqlx::{Error, PgPool};

pub async fn lead_earliest_mention_time(
    pool: &PgPool,
    chat_id: i64,
) -> Result<NaiveDateTime, Error> {
    sqlx::query_as(
        "SELECT updated_at FROM mentions \
                WHERE chat_id = $1 \
                    ORDER BY updated_at DESC LIMIT 1",
    )
    .bind(chat_id)
    .fetch_optional(pool)
    .await
    .map(|v: Option<(NaiveDateTime,)>| v.unwrap_or((NaiveDateTime::MAX,)).0)
}

pub async fn insert_mention(
    pool: &PgPool,
    user_id: i64,
    username: &str,
    chat_id: i64,
) -> Result<PgQueryResult, Error> {
    sqlx::query(
        "INSERT INTO mentions(user_id, username, chat_id) VALUES ($1, $2, $3)  \
                ON CONFLICT (user_id, chat_id) DO UPDATE \
                    SET updated_at = current_timestamp, counter = mentions.counter + 1",
    )
    .bind(user_id)
    .bind(username)
    .bind(chat_id)
    .execute(pool)
    .await
}
