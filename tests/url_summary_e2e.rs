mod common;

use common::*;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test(flavor = "multi_thread")]
async fn url_summary_fetches_article_summarizes_and_replies() {
    let pg = spawn_postgres().await;
    let redis = spawn_redis().await;
    let (telegram, bot) = spawn_telegram().await;
    let canned_summary = "Краткое содержание статьи.";
    let (openai, openai_url) = spawn_openai(canned_summary).await;
    let gpt = gpt_parameters(redis.connection_manager.clone(), openai_url);

    let article = MockServer::start().await;
    let long_text = "lorem ipsum dolor sit amet ".repeat(80);
    let html = format!("<html><body><p>{long_text}</p></body></html>");
    Mock::given(method("GET"))
        .and(path("/post/42"))
        .respond_with(ResponseTemplate::new(200).set_body_string(html))
        .mount(&article)
        .await;

    let chat_id = -1003000_i64;
    let user_id = 22_i64;
    let article_url = format!("{}/post/42", article.uri());
    let text = format!("посмотри {article_url}");

    let update = text_message_update(&text, chat_id, user_id, 1);
    dispatch_one(bot, pg.pool.clone(), gpt, update).await;

    let article_calls = article
        .received_requests()
        .await
        .expect("collect article requests");
    assert_eq!(article_calls.len(), 1, "expected 1 article fetch");

    let openai_calls = openai
        .received_requests()
        .await
        .expect("collect openai requests");
    assert_eq!(openai_calls.len(), 1, "expected 1 openai call");

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
    let body = &send_message_bodies[0];
    assert!(body.contains("TLDR"), "sendMessage body: {body}");
    assert!(body.contains(canned_summary), "sendMessage body: {body}");
}
