//! End-to-end coverage for the handler routes the existing e2e suite did not
//! reach: the blazing-fast mention, the gayness mention (restrict + reply), and
//! the reply-to-a-bot-message path through `handle_reply`.

mod common;

use common::*;
use rust_bot::chat_gpt_handler::BotProfile;
use rust_bot::chat_repository;

fn send_message_bodies(requests: &[wiremock::Request]) -> Vec<String> {
    requests
        .iter()
        .filter(|r| r.url.path().ends_with("/SendMessage"))
        .map(|r| String::from_utf8_lossy(&r.body).to_string())
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn blazing_fast_mention_replies() {
    let pg = spawn_postgres().await;
    let redis = spawn_redis().await;
    let (telegram, bot) = spawn_telegram().await;
    let (_openai, openai_url) = spawn_openai("unused").await;
    let gpt = gpt_parameters(redis.connection_manager.clone(), openai_url);

    // "blazing fast" matches BLAZING_FAST_REGEX and no earlier route.
    let update = text_message_update("blazing fast is a myth", -1_005_000, 55, 1);
    dispatch_one(bot, pg.pool.clone(), gpt, update).await;

    let requests = telegram
        .received_requests()
        .await
        .expect("collect telegram requests");
    let bodies = send_message_bodies(&requests);
    assert_eq!(bodies.len(), 1, "expected one reply, got {}", bodies.len());
    assert!(
        bodies[0].contains("Did you mean Rust"),
        "sendMessage body: {}",
        bodies[0]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn gayness_mention_restricts_and_replies() {
    let pg = spawn_postgres().await;
    let redis = spawn_redis().await;
    let (telegram, bot) = spawn_telegram().await;
    let (_openai, openai_url) = spawn_openai("unused").await;
    let gpt = gpt_parameters(redis.connection_manager.clone(), openai_url);

    // "I am 3% gay" matches GAYNESS_REGEX (the " 3% g" substring).
    let update = text_message_update("I am 3% gay", -1_006_000, 66, 1);
    dispatch_one(bot, pg.pool.clone(), gpt, update).await;

    let requests = telegram
        .received_requests()
        .await
        .expect("collect telegram requests");

    let restrict_count = requests
        .iter()
        .filter(|r| r.url.path().ends_with("/RestrictChatMember"))
        .count();
    assert_eq!(restrict_count, 1, "expected one restrictChatMember call");

    let bodies = send_message_bodies(&requests);
    assert_eq!(bodies.len(), 1, "expected one reply, got {}", bodies.len());
    assert!(
        bodies[0].contains("mute"),
        "sendMessage body: {}",
        bodies[0]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn reply_to_bot_message_routes_to_gpt() {
    let pg = spawn_postgres().await;
    let redis = spawn_redis().await;
    let (telegram, bot) = spawn_telegram().await;
    let canned = "И тебе не хворать.";
    let (openai, openai_url) = spawn_openai(canned).await;
    let gpt = gpt_parameters(redis.connection_manager.clone(), openai_url);

    let chat_id = -1_007_000_i64;
    let bot_msg_id = 500_i32;

    // Seed: the replied-to message was sent under the Fedor profile. Use the
    // same key format the handler builds (`chat:{chat_id:#?}`).
    let mut cm = redis.connection_manager.clone();
    let chat_key = format!("chat:{chat_id:#?}");
    chat_repository::push_bot_msg_identifier(&mut cm, &chat_key, bot_msg_id, BotProfile::Fedor)
        .await
        .expect("seed bot msg profile");

    // Neutral text (no keyword) replying to that bot message -> handle_reply.
    let update = reply_message_update("спасибо", chat_id, 77, 1, bot_msg_id);
    dispatch_one(bot, pg.pool.clone(), gpt, update).await;

    let openai_calls = openai
        .received_requests()
        .await
        .expect("collect openai requests");
    assert_eq!(openai_calls.len(), 1, "reply should trigger one gpt call");

    let requests = telegram
        .received_requests()
        .await
        .expect("collect telegram requests");
    let bodies = send_message_bodies(&requests);
    assert_eq!(bodies.len(), 1, "expected one reply, got {}", bodies.len());
    assert!(
        bodies[0].contains(canned),
        "sendMessage body: {}",
        bodies[0]
    );
}
