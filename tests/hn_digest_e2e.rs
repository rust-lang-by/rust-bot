mod common;

use chrono::NaiveTime;
use common::*;
use rust_bot::hn_digest::{run_digest, HnDigestConfig};
use serde_json::{json, Value};
use teloxide::types::ChatId;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn mount_json(server: &MockServer, at: &str, body: Value) {
    Mock::given(method("GET"))
        .and(path(at))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}

async fn spawn_articles() -> MockServer {
    let server = MockServer::start().await;
    let html = format!(
        "<html><body><p>{}</p></body></html>",
        "lorem ipsum dolor sit amet ".repeat(80)
    );
    for article in ["/a", "/b"] {
        Mock::given(method("GET"))
            .and(path(article))
            .respond_with(ResponseTemplate::new(200).set_body_raw(html.clone(), "text/html"))
            .mount(&server)
            .await;
    }
    Mock::given(method("GET"))
        .and(path("/broken"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    server
}

/// Front page: a linked story, an Ask HN post, a story whose article 404s,
/// a job post, then a second linked story. With top_n = 2 the digest must
/// pick stories 101 and 105.
async fn spawn_hn(articles: &MockServer) -> MockServer {
    let hn = MockServer::start().await;
    let a = articles.uri();
    mount_json(
        &hn,
        "/v0/topstories.json",
        json!([101, 102, 103, 104, 105, 106]),
    )
    .await;
    let items = [
        json!({"id": 101, "type": "story", "title": "Rust <3 & you", "url": format!("{a}/a"), "score": 456, "descendants": 123}),
        json!({"id": 102, "type": "story", "title": "Ask HN: anything?", "score": 50, "descendants": 9}),
        json!({"id": 103, "type": "story", "title": "Gone", "url": format!("{a}/broken"), "score": 40, "descendants": 1}),
        json!({"id": 104, "type": "job", "title": "Hiring", "url": format!("{a}/a")}),
        json!({"id": 105, "type": "story", "title": "Second story", "url": format!("{a}/b"), "score": 7, "descendants": 2}),
        json!({"id": 106, "type": "story", "title": "Not needed", "url": format!("{a}/a")}),
    ];
    for item in items {
        let id = item["id"].as_u64().expect("item id");
        mount_json(&hn, &format!("/v0/item/{id}.json"), item).await;
    }
    hn
}

#[tokio::test(flavor = "multi_thread")]
async fn hn_digest_summarizes_once_and_posts_to_every_chat() {
    let redis = spawn_redis().await;
    let (telegram, bot) = spawn_telegram().await;
    let (openai, openai_url) = spawn_openai("- <b>bold</b> & brief").await;
    let gpt = gpt_parameters(redis.connection_manager.clone(), openai_url);
    let articles = spawn_articles().await;
    let hn = spawn_hn(&articles).await;

    let chats = [-1007001_i64, -1007002_i64];
    let config = HnDigestConfig {
        chat_ids: chats.iter().map(|&id| ChatId(id)).collect(),
        time_utc: NaiveTime::MIN,
        top_n: 2,
        run_on_startup: false,
        hn_api_base_url: format!("{}/v0", hn.uri()),
    };

    run_digest(&bot, &gpt, &config).await;

    let openai_calls = openai
        .received_requests()
        .await
        .expect("collect openai requests");
    assert_eq!(
        openai_calls.len(),
        2,
        "each story is summarized once regardless of chat count"
    );

    let sent: Vec<Value> = telegram
        .received_requests()
        .await
        .expect("collect telegram requests")
        .iter()
        .filter(|r| r.url.path().ends_with("/SendMessage"))
        .map(|r| serde_json::from_slice(&r.body).expect("sendMessage body is JSON"))
        .collect();
    assert_eq!(sent.len(), 4, "2 stories x 2 chats: {sent:#?}");

    for (chat_index, chat_id) in chats.iter().enumerate() {
        let first = &sent[chat_index * 2];
        let second = &sent[chat_index * 2 + 1];
        for body in [first, second] {
            assert_eq!(body["chat_id"], json!(chat_id), "{body:#?}");
            assert_eq!(body["parse_mode"], json!("HTML"), "{body:#?}");
        }

        let first_text = first["text"].as_str().expect("text");
        assert!(first_text.starts_with("#1 "), "{first_text}");
        assert!(
            first_text.contains("Rust &lt;3 &amp; you</a>"),
            "{first_text}"
        );
        assert!(first_text.contains("▲ 456"), "{first_text}");
        assert!(
            first_text.contains("news.ycombinator.com/item?id=101\">123 comments</a>"),
            "{first_text}"
        );
        assert!(
            first_text.contains("&lt;b&gt;bold&lt;/b&gt; &amp; brief"),
            "{first_text}"
        );

        let second_text = second["text"].as_str().expect("text");
        assert!(second_text.starts_with("#2 "), "{second_text}");
        assert!(second_text.contains("Second story</a>"), "{second_text}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn hn_digest_posts_nothing_when_top_stories_unavailable() {
    let redis = spawn_redis().await;
    let (telegram, bot) = spawn_telegram().await;
    let (openai, openai_url) = spawn_openai("unused").await;
    let gpt = gpt_parameters(redis.connection_manager.clone(), openai_url);
    let hn = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v0/topstories.json"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&hn)
        .await;

    let config = HnDigestConfig {
        chat_ids: vec![ChatId(-1007003)],
        time_utc: NaiveTime::MIN,
        top_n: 3,
        run_on_startup: false,
        hn_api_base_url: format!("{}/v0", hn.uri()),
    };

    run_digest(&bot, &gpt, &config).await;

    assert!(openai
        .received_requests()
        .await
        .expect("collect openai requests")
        .is_empty());
    let send_count = telegram
        .received_requests()
        .await
        .expect("collect telegram requests")
        .iter()
        .filter(|r| r.url.path().ends_with("/SendMessage"))
        .count();
    assert_eq!(send_count, 0);
}
