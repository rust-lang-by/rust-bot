use std::env;
use std::sync::Arc;

use log::info;
use redis::aio::ConnectionManager;
use sqlx::PgPool;
use teloxide::prelude::*;

use rust_bot::{AppDeps, GptParameters, MentionParameters, DEFAULT_OPENAI_BASE_URL};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    info!("Starting bot...");

    let bot = Bot::from_env();
    let db_pool = establish_connection().await;
    let chat_gpt_api_token =
        env::var("CHAT_GPT_API_TOKEN").expect("CHAT_GPT_API_TOKEN must be set");
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set");
    let redis_client = redis::Client::open(redis_url).unwrap();
    let redis_connection_manager = ConnectionManager::new(redis_client).await.unwrap();

    let gpt_parameters = GptParameters {
        chat_gpt_api_token: Arc::from(chat_gpt_api_token),
        openai_base_url: Arc::from(DEFAULT_OPENAI_BASE_URL),
        http_client: reqwest::Client::new(),
        redis_connection_manager,
    };

    let deps = AppDeps {
        bot,
        db_pool,
        gpt_parameters,
        mention_parameters: MentionParameters::default(),
    };

    rust_bot::run(deps).await;
}

async fn establish_connection() -> PgPool {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgPool::connect(&database_url)
        .await
        .expect("Can't establish connection")
}
