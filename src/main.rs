use std::env;

use chrono::Duration;
use log::info;
use regex::Regex;
use sqlx::{PgPool, Pool, Postgres};
use teloxide::prelude::*;

mod bf_mention_handler;
mod chat_gpt_handler;
mod mention_repository;
mod rust_mention_handler;

const RUST_REGEX: &str = r"(?i)(rust|раст)";
const BLAZING_FAST_REGEX: &str = r"\w*[BbБб][LlЛл]\w*\W[FfФф][AaАа]\w*\b";
const CHAT_GPT_REGEX: &str = r"(?i)(fedor|федор|felix|феликс|feris|ferris|ферис|феррис)";
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
    let mention_parameters = MentionParameters {
        rust_regex: Regex::new(RUST_REGEX).expect("Can't compile regex"),
        blazing_fast_regex: Regex::new(BLAZING_FAST_REGEX).expect("Can't compile regex"),
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
                 chat_gpt_api_token: String,
                 bot: Bot| async move {
                    if let Some(message) = msg.text() {
                        match message {
                            m if mention_parameters.chat_gpt_regex.is_match(m) => {
                                chat_gpt_handler::handle_chat_gpt_question(
                                    bot,
                                    msg,
                                    chat_gpt_api_token,
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
                            _ => {}
                        }
                    }
                    respond(())
                },
            ),
    );
    Dispatcher::builder(bot, handler)
        // Here you specify initial dependencies that all handlers will receive
        .dependencies(dptree::deps![
            mention_parameters,
            db_pool,
            chat_gpt_api_token
        ])
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
    chat_gpt_regex: Regex,
    req_time_diff: Duration,
}

#[cfg(test)]
mod tests {
    use crate::CHAT_GPT_REGEX;
    use regex::Regex;

    #[test]
    fn test_chat_gpt_regex() {
        let chat_gpt_regex = Regex::new(CHAT_GPT_REGEX).expect("Can't compile regex");
        assert!(chat_gpt_regex.is_match("ухх Федор как дела?"));
        assert!(chat_gpt_regex.is_match("pFedor tests"));
        assert!(chat_gpt_regex.is_match("p Felix greate"));
        assert!(chat_gpt_regex.is_match("Феликс"));
        assert!(chat_gpt_regex.is_match("[[[Ferris"));
    }
}
