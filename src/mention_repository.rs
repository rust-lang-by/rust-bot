use sqlx::postgres::PgQueryResult;
use sqlx::types::chrono::NaiveDateTime;
use sqlx::PgPool;

pub async fn lead_earliest_mention_time(
    pool: &PgPool,
    chat_id: i64,
) -> NaiveDateTime {
    sqlx::query_as(
        "SELECT updated_at FROM mentions \
                WHERE chat_id = $1 \
                    ORDER BY updated_at DESC LIMIT 1",
    )
    .bind(chat_id)
    .fetch_optional(pool)
    .await
    .map(|v: Option<(NaiveDateTime,)>| v.unwrap_or((NaiveDateTime::MIN,)).0)
    .expect("Can't pool latest mention time")
}

pub async fn insert_mention(
    pool: &PgPool,
    user_id: i64,
    username: &str,
    chat_id: i64,
) -> PgQueryResult {
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
    .expect("Can't insert mention")
}
