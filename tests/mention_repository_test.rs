//! Direct coverage of `mention_repository` against a real Postgres
//! (testcontainers), exercising the insert / lead-earliest-mention-time queries
//! and the `ON CONFLICT` upsert that the rust-mention handler relies on.

mod common;

use common::spawn_postgres;
use rust_bot::mention_repository;
use sqlx::types::chrono::NaiveDateTime;

#[tokio::test(flavor = "multi_thread")]
async fn insert_upserts_and_lead_time_reflects_state() {
    let pg = spawn_postgres().await;
    let chat_id = -4242_i64;
    let user_id = 7_i64;

    // No rows for this chat yet -> the query returns the MAX sentinel.
    let empty = mention_repository::lead_earliest_mention_time(&pg.pool, chat_id)
        .await
        .expect("lead time on empty table");
    assert_eq!(
        empty,
        NaiveDateTime::MAX,
        "empty chat should yield the sentinel"
    );

    // First insert creates exactly one row with counter = 1 (column default).
    mention_repository::insert_mention(&pg.pool, user_id, "alice", chat_id)
        .await
        .expect("first insert");

    let (count, counter) = row_stats(&pg.pool, user_id, chat_id).await;
    assert_eq!(count, 1, "one row after first insert");
    assert_eq!(counter, 1, "counter starts at 1");

    // lead time now returns a real timestamp, not the sentinel.
    let after_insert = mention_repository::lead_earliest_mention_time(&pg.pool, chat_id)
        .await
        .expect("lead time after insert");
    assert_ne!(
        after_insert,
        NaiveDateTime::MAX,
        "should return a real timestamp"
    );

    // Re-inserting the same (user, chat) upserts: still one row, counter bumped.
    mention_repository::insert_mention(&pg.pool, user_id, "alice", chat_id)
        .await
        .expect("second insert (conflict)");

    let (count2, counter2) = row_stats(&pg.pool, user_id, chat_id).await;
    assert_eq!(count2, 1, "ON CONFLICT should update, not add a row");
    assert_eq!(counter2, 2, "counter should increment on conflict");
}

async fn row_stats(pool: &sqlx::PgPool, user_id: i64, chat_id: i64) -> (i64, i32) {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM mentions WHERE user_id = $1 AND chat_id = $2")
            .bind(user_id)
            .bind(chat_id)
            .fetch_one(pool)
            .await
            .expect("count rows");
    let counter: i32 =
        sqlx::query_scalar("SELECT counter FROM mentions WHERE user_id = $1 AND chat_id = $2")
            .bind(user_id)
            .bind(chat_id)
            .fetch_one(pool)
            .await
            .expect("read counter");
    (count, counter)
}
