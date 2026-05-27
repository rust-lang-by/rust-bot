mod common;

use common::*;
use redis::AsyncCommands;

#[tokio::test(flavor = "multi_thread")]
async fn chat_gpt_routes_to_openai_and_writes_redis_context() {
    let pg = spawn_postgres().await;
    let redis = spawn_redis().await;
    let (telegram, bot) = spawn_telegram().await;
    let canned = "Привет, дружище!";
    let (openai, openai_url) = spawn_openai(canned).await;
    let gpt = gpt_parameters(redis.connection_manager.clone(), openai_url);

    let chat_id = -1002000_i64;
    let user_id = 11_i64;
    let text = "fedor, привет";

    let update = text_message_update(text, chat_id, user_id, 1);
    dispatch_one(bot, pg.pool.clone(), gpt, update).await;

    let openai_calls = openai
        .received_requests()
        .await
        .expect("collect openai requests");
    assert_eq!(openai_calls.len(), 1, "expected 1 openai call");
    let openai_body = String::from_utf8_lossy(&openai_calls[0].body);
    assert!(
        openai_body.contains("Федор"),
        "openai body missing Fedor system context: {openai_body}"
    );
    assert!(
        openai_body.contains("fedor"),
        "openai body missing user text: {openai_body}"
    );

    let telegram_requests = telegram
        .received_requests()
        .await
        .expect("collect telegram requests");
    let send_message_bodies: Vec<String> = telegram_requests
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
    assert!(
        send_message_bodies[0].contains(canned),
        "sendMessage body missing canned reply: {}",
        send_message_bodies[0]
    );

    let mut cm = redis.connection_manager.clone();
    let key = format!("Fedor:chat:{chat_id}");
    let entries: Vec<String> = cm.lrange(&key, 0, -1).await.expect("redis lrange");
    assert_eq!(
        entries.len(),
        2,
        "expected 2 context entries (user + assistant), got {}: {:?}",
        entries.len(),
        entries
    );
}
