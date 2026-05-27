#![allow(dead_code)]

use std::sync::Arc;
use std::time::Duration;

use redis::aio::ConnectionManager;
use serde_json::{json, Value};
use sqlx::PgPool;
use teloxide::prelude::*;
use teloxide::types::Update;
use testcontainers::runners::AsyncRunner;
use testcontainers::ContainerAsync;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::redis::Redis;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use rust_bot::{build_handler, GptParameters, MentionParameters};

pub const TEST_BOT_TOKEN: &str = "test-token";

pub struct PostgresHarness {
    pub _container: ContainerAsync<Postgres>,
    pub pool: PgPool,
}

pub struct RedisHarness {
    pub _container: ContainerAsync<Redis>,
    pub connection_manager: ConnectionManager,
    pub url: String,
}

pub async fn spawn_postgres() -> PostgresHarness {
    let container = Postgres::default()
        .start()
        .await
        .expect("postgres container start");
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("postgres port");
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");

    let pool = PgPool::connect(&url).await.expect("connect postgres");
    run_migrations(&pool).await;

    PostgresHarness {
        _container: container,
        pool,
    }
}

pub async fn spawn_redis() -> RedisHarness {
    let container = Redis::default()
        .start()
        .await
        .expect("redis container start");
    let port = container
        .get_host_port_ipv4(6379)
        .await
        .expect("redis port");
    let url = format!("redis://127.0.0.1:{port}");

    let client = redis::Client::open(url.clone()).expect("redis client");
    let connection_manager = ConnectionManager::new(client)
        .await
        .expect("redis connection manager");

    RedisHarness {
        _container: container,
        connection_manager,
        url,
    }
}

/// Migrations are run by hand because the on-disk file
/// `202105011313650_mentions_counter.sql` has a 15-digit timestamp (typo) that
/// sorts later than `20230310100000_*` under integer ordering, and the last
/// migration ends with a stray `END` token. Both issues are tolerated in
/// production (DB is already migrated) but break `sqlx::migrate!`.
async fn run_migrations(pool: &PgPool) {
    let migrations = [
        include_str!("../../migration/20200924213650_mentions.sql"),
        include_str!("../../migration/202105011313650_mentions_counter.sql"),
        include_str!("../../migration/20230310100000_mensions_chat_id.sql"),
    ];
    for sql in migrations {
        for stmt in sql.split(';') {
            let trimmed = stmt.trim();
            if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("END") {
                continue;
            }
            sqlx::query(trimmed)
                .execute(pool)
                .await
                .unwrap_or_else(|e| panic!("migration stmt failed: {trimmed}\n{e}"));
        }
    }
}

/// Returns a wiremock `MockServer` doubling as the Telegram Bot API plus a
/// `Bot` already pointed at it. Pre-registers `sendMessage` and `sendSticker`
/// to return a minimal successful response so handlers don't fail on unmocked
/// outbound calls.
pub async fn spawn_telegram() -> (MockServer, Bot) {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/bot{TEST_BOT_TOKEN}/sendMessage")))
        .respond_with(ResponseTemplate::new(200).set_body_json(default_message_response()))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path(format!("/bot{TEST_BOT_TOKEN}/sendSticker")))
        .respond_with(ResponseTemplate::new(200).set_body_json(default_message_response()))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path(format!("/bot{TEST_BOT_TOKEN}/restrictChatMember")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true, "result": true})))
        .mount(&server)
        .await;

    let url = reqwest::Url::parse(&server.uri()).expect("parse telegram mock uri");
    let bot = Bot::new(TEST_BOT_TOKEN).set_api_url(url);
    (server, bot)
}

/// Spins a wiremock OpenAI that returns `canned_reply` from
/// `/v1/chat/completions`.
pub async fn spawn_openai(canned_reply: &str) -> (MockServer, String) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl-test",
            "object": "chat.completion",
            "created": 1700000000_u64,
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": canned_reply},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
        })))
        .mount(&server)
        .await;
    let base_url = format!("{}/v1/chat/completions", server.uri());
    (server, base_url)
}

pub fn gpt_parameters(redis: ConnectionManager, openai_base_url: String) -> GptParameters {
    GptParameters {
        chat_gpt_api_token: Arc::from("test-openai-token"),
        openai_base_url: Arc::from(openai_base_url),
        http_client: reqwest::Client::new(),
        redis_connection_manager: redis,
    }
}

pub fn text_message_update(text: &str, chat_id: i64, user_id: i64, message_id: i32) -> Update {
    let now = chrono::Utc::now().timestamp();
    let value: Value = json!({
        "update_id": message_id,
        "message": {
            "message_id": message_id,
            "date": now,
            "chat": {
                "id": chat_id,
                "type": "supergroup",
                "title": "test-chat"
            },
            "from": {
                "id": user_id,
                "is_bot": false,
                "first_name": "Alice",
                "username": "alice"
            },
            "text": text,
            "entities": []
        }
    });
    serde_json::from_value(value).expect("build Update")
}

/// Build deps, dispatch a single update through the real handler tree, and
/// fail fast if anything stalls.
pub async fn dispatch_one(bot: Bot, pool: PgPool, gpt_parameters: GptParameters, update: Update) {
    let handler = build_handler();
    let mention_parameters = MentionParameters::default();
    let deps = dptree::deps![update, bot, mention_parameters, pool, gpt_parameters];
    let _ = tokio::time::timeout(Duration::from_secs(15), handler.dispatch(deps))
        .await
        .expect("dispatcher did not complete within 15s");
}

fn default_message_response() -> Value {
    json!({
        "ok": true,
        "result": {
            "message_id": 100,
            "date": 1700000000_u64,
            "chat": {"id": -1001000, "type": "supergroup", "title": "test-chat"},
            "from": {"id": 1, "is_bot": true, "first_name": "TestBot", "username": "test_bot"},
            "text": "ok"
        }
    })
}
