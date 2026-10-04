#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::env;
use std::sync::Arc;

use anyhow::Context;
use log::{error, info, warn};
use redis::aio::ConnectionManager;
use sqlx::PgPool;
use teloxide::prelude::*;
use tokio_util::sync::CancellationToken;

use rust_bot::hn_digest::{self, HnDigestConfig, HnDigestParameters};
use rust_bot::{AppDeps, GptParameters, MentionParameters, DEFAULT_OPENAI_BASE_URL};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();
    info!("Starting bot...");

    let telegram_token = env::var("TELOXIDE_TOKEN").context("TELOXIDE_TOKEN must be set")?;
    let bot = Bot::new(telegram_token);
    let db_pool = establish_connection().await?;
    let chat_gpt_api_token =
        env::var("CHAT_GPT_API_TOKEN").context("CHAT_GPT_API_TOKEN must be set")?;
    let redis_url = env::var("REDIS_URL").context("REDIS_URL must be set")?;
    let redis_client =
        redis::Client::open(redis_url).context("failed to open Redis client from REDIS_URL")?;
    let redis_connection_manager = ConnectionManager::new(redis_client)
        .await
        .context("failed to connect to Redis")?;

    let gpt_parameters = GptParameters {
        chat_gpt_api_token: Arc::from(chat_gpt_api_token),
        openai_base_url: Arc::from(DEFAULT_OPENAI_BASE_URL),
        http_client: reqwest::Client::new(),
        redis_connection_manager,
    };

    let shutdown = CancellationToken::new();
    let hn_digest_task = match HnDigestConfig::from_env() {
        Some(config) => Some(hn_digest::spawn_scheduler(
            bot.clone(),
            gpt_parameters.clone(),
            config,
            shutdown.clone(),
        )),
        None => {
            info!("HN digest disabled: HN_DIGEST_CHAT_IDS is not set");
            None
        }
    };

    let deps = AppDeps {
        bot,
        db_pool,
        gpt_parameters,
        mention_parameters: MentionParameters::default(),
        hn_digest_parameters: HnDigestParameters::default(),
    };

    let bot_run = rust_bot::run(deps);
    let Some(mut hn_digest_task) = hn_digest_task else {
        return bot_run.await;
    };
    tokio::pin!(bot_run);
    tokio::select! {
        result = &mut bot_run => {
            shutdown.cancel();
            if let Err(err) = hn_digest_task.await {
                error!("HN digest scheduler failed: {err}");
            }
            result
        }
        joined = &mut hn_digest_task => {
            // The scheduler only ends early by panicking; keep serving chats.
            match joined {
                Err(err) => error!("HN digest scheduler crashed, no digests until restart: {err}"),
                Ok(()) => warn!("HN digest scheduler exited unexpectedly"),
            }
            bot_run.await
        }
    }
}

async fn establish_connection() -> anyhow::Result<PgPool> {
    let database_url = env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    PgPool::connect(&database_url)
        .await
        .context("failed to connect to Postgres")
}
