use std::env;

use chrono::Duration;
use log::info;
use redis::aio::ConnectionManager;
use regex::Regex;
use sqlx::{PgPool, Pool, Postgres};
use teloxide::prelude::*;

mod bf_mention_handler;
mod chat_gpt_handler;
mod chat_repository;
mod gayness_handler;
mod mention_repository;
mod rust_mention_handler;

const RUST_REGEX: &str = r"(?i)(rust|раст)(.\W|.$|\W|$)";
const BLAZING_FAST_REGEX: &str = r"\w*[BbБб][LlЛл]\w*\W[FfФф][AaАа]\w*\b";
const GAYNESS_REGEX: &str = r"(\D[0-4]|\D)\d%\Dg";
const CHAT_GPT_REGEX: &str = r"(?i)(fedor|ф[её]дор|федя|felix|феликс|feris|ferris|ферис|феррис)";
const MIN_TIME_DIFF: i64 = 15;

#[tokio::main]
async fn main() {
    run().await;
}

async fn run() {
    pretty_env_logger::init();
    info!("Starting bot...");

    let bot = Bot::from_env();
    let db_pool = establish_connection().await;
    let chat_gpt_api_token =
        env::var("CHAT_GPT_API_TOKEN").expect("CHAT_GPT_API_TOKEN must be set");
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set");
    let redis_client = redis::Client::open(redis_url).unwrap();
    let redis_connection_manager = ConnectionManager::new(redis_client.clone()).await.unwrap();

    let gpt_parameters = GPTParameters {
        chat_gpt_api_token,
        redis_connection_manager,
    };
    let mention_parameters = MentionParameters {
        rust_regex: Regex::new(RUST_REGEX).expect("Can't compile regex"),
        blazing_fast_regex: Regex::new(BLAZING_FAST_REGEX).expect("Can't compile regex"),
        gayness_regex: Regex::new(GAYNESS_REGEX).expect("Can't compile regex"),
        chat_gpt_regex: Regex::new(CHAT_GPT_REGEX).expect("Can't compile regex"),
        req_time_diff: Duration::minutes(MIN_TIME_DIFF),
    };

    let handler = Update::filter_message().branch(
        // Filtering to focus on chat mentions
        dptree::filter(|msg: Message| !msg.chat.is_private())
            // An endpoint is the last update handler.
            .endpoint(
                |msg: Message,
                 mention_parameters: MentionParameters,
                 db_pool: Pool<Postgres>,
                 mut gpt_parameters: GPTParameters,
                 bot: Bot| async move {
                    if let Some(message) = msg.text() {
                        match message {
                            m if mention_parameters.chat_gpt_regex.is_match(m) => {
                                chat_gpt_handler::handle_chat_gpt_question(
                                    bot,
                                    msg,
                                    &mut gpt_parameters,
                                )
                                .await
                            }
                            m if mention_parameters.rust_regex.is_match(m) => {
                                rust_mention_handler::handle_rust_matched_mention(
                                    bot,
                                    msg,
                                    db_pool,
                                    mention_parameters.req_time_diff,
                                )
                                .await
                            }
                            m if mention_parameters.blazing_fast_regex.is_match(m) => {
                                bf_mention_handler::handle_bf_matched_mention(bot, msg).await
                            }
                            m if mention_parameters.gayness_regex.is_match(m) => {
                                gayness_handler::handle_gayness_mention(bot, msg).await
                            }
                            _ => {
                                if let Some(reply_msg) = &msg.reply_to_message() {
                                    chat_gpt_handler::handle_reply(
                                        &bot,
                                        &msg,
                                        reply_msg,
                                        &mut gpt_parameters,
                                    )
                                    .await;
                                }
                            }
                        }
                    }

                    respond(())
                },
            ),
    );
    Dispatcher::builder(bot, handler)
        // Here you specify initial dependencies that all handlers will receive
        .dependencies(dptree::deps![mention_parameters, db_pool, gpt_parameters])
        // If the dispatcher fails for some reason, execute this handler.
        .error_handler(LoggingErrorHandler::with_custom_text(
            "An error has occurred in the dispatcher",
        ))
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

async fn establish_connection() -> Pool<Postgres> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgPool::connect(&database_url)
        .await
        .expect("Can't establish connection")
}

#[derive(Clone)]
struct MentionParameters {
    rust_regex: Regex,
    blazing_fast_regex: Regex,
    gayness_regex: Regex,
    chat_gpt_regex: Regex,
    req_time_diff: Duration,
}

#[derive(Clone)]
pub struct GPTParameters {
    chat_gpt_api_token: String,
    redis_connection_manager: ConnectionManager,
}

#[cfg(test)]
mod tests {
    use crate::{CHAT_GPT_REGEX, RUST_REGEX};
    use regex::Regex;

    #[test]
    fn test_rust_gpt_regex() {
        let chat_gpt_regex = Regex::new(RUST_REGEX).expect("Can't compile regex");
        assert!(chat_gpt_regex.is_match("test rust test"));
        assert!(chat_gpt_regex.is_match("RusT"));
        assert!(chat_gpt_regex.is_match("что там у раста"));
        assert!(chat_gpt_regex.is_match("чэ тупо раст тэст"));
    }

    #[test]
    fn test_chat_gpt_regex() {
        let chat_gpt_regex = Regex::new(CHAT_GPT_REGEX).expect("Can't compile regex");
        assert!(chat_gpt_regex.is_match("ухх Федор как дела?"));
        assert!(chat_gpt_regex.is_match("pFedor tests"));
        assert!(chat_gpt_regex.is_match("p Felix greate"));
        assert!(chat_gpt_regex.is_match("Феликс"));
        assert!(chat_gpt_regex.is_match("[[[Ferris"));
        assert!(chat_gpt_regex.is_match("[ Фёдор ъ"));
    }
}
