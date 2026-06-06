mod common;

use common::*;

#[tokio::test(flavor = "multi_thread")]
async fn rust_mention_triggers_reply_and_inserts_mention() {
    let pg = spawn_postgres().await;
    let redis = spawn_redis().await;
    let (telegram, bot) = spawn_telegram().await;
    let (_openai, openai_url) = spawn_openai("not used").await;
    let gpt = gpt_parameters(redis.connection_manager.clone(), openai_url);

    let chat_id = -1001000_i64;
    let user_id = 42_i64;

    sqlx::query(
        "INSERT INTO mentions(user_id, username, chat_id, updated_at) \
         VALUES ($1, $2, $3, NOW() - INTERVAL '1 hour')",
    )
    .bind(99_i64)
    .bind("seed")
    .bind(chat_id)
    .execute(&pg.pool)
    .await
    .expect("seed mention row");

    let update = text_message_update("Rust is great", chat_id, user_id, 1);
    dispatch_one(bot, pg.pool.clone(), gpt, update).await;

    let requests = telegram
        .received_requests()
        .await
        .expect("collect telegram requests");

    let send_message_bodies: Vec<String> = requests
        .iter()
        .filter(|r| r.url.path().ends_with("/SendMessage"))
        .map(|r| String::from_utf8_lossy(&r.body).to_string())
        .collect();

    assert_eq!(
        send_message_bodies.len(),
        1,
        "expected 1 sendMessage call, got {}",
        send_message_bodies.len()
    );
    let body = &send_message_bodies[0];
    assert!(body.contains("Hi, alice!"), "sendMessage body: {body}");
    assert!(
        body.contains("since last incident"),
        "sendMessage body: {body}"
    );

    let send_sticker_count = requests
        .iter()
        .filter(|r| r.url.path().ends_with("/SendSticker"))
        .count();
    assert_eq!(send_sticker_count, 1, "expected 1 sendSticker call");

    let alice_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mentions WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&pg.pool)
        .await
        .expect("count alice mentions");
    assert_eq!(alice_count, 1, "alice should have one mention row");
}
