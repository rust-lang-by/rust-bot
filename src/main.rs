use std::env;

use chrono::{DateTime, Duration, NaiveDateTime, TimeZone, Utc};
use log::{error, info};
use regex::Regex;
use sqlx::{PgPool, Pool, Postgres};
use teloxide::prelude::*;
use teloxide::types::MessageKind::Common;
use teloxide::types::{ChatId, InputFile, MessageCommon, MessageId, User};

mod mention_repository;

const STICKERS: &[&str; 5] = &[
    "CAACAgEAAx0CTdy33AAD3mQO6sc3rzklybqG4MMI4MLXpXJIAAKCAQACaXoxBT0NGBN6KJNELwQ",
    "CAACAgEAAx0CTdy33AACAQFkF6KoodtDg4KfcPHlUk_7SRFN7QACkQEAAml6MQW86C1JCZcTkS8E",
    "CAACAgEAAx0CTdy33AACAQxkF6Mu7nPaIs9rmMBfXs71BBPxfgACnAEAAml6MQVkU_PxsG8GmS8E",
    "CAACAgEAAx0CTdy33AACARFkF6RG5xa8L2rn6ENe3NsMktY7GgACaQEAAml6MQWdvTv0FuDgLC8E",
    "CAACAgEAAx0CTdy33AACARNkF6SmiAABryW7RcGozvQDCys7JNUAAlcBAAJpejEFk0uf6g86yKAvBA",
];
const HOURS_PER_DAY: i64 = 24;
const MINUTES_PER_HOUR: i64 = 60;
const MIN_TIME_DIFF: i64 = 15;
const RUST_REGEX: &str = r"\b[RrРр][AaUuАа][CcSsСс][TtТт]\b";
const BLAZING_FAST_REGEX: &str = r"\b[BbБб][LlЛл]\w*\W[FfФф][AaАа]\w*\b";

#[tokio::main]
async fn main() {
    run().await;
}

async fn run() {
    pretty_env_logger::init();
    info!("Starting bot...");

    let bot = Bot::from_env();
    let db_pool = establish_connection().await;
    let mention_parameters = MentionParameters {
        rust_regex: Regex::new(RUST_REGEX).expect("Can't compile regex"),
        blazing_fast_regex: Regex::new(BLAZING_FAST_REGEX).expect("Can't compile regex"),
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
                 bot: Bot| async move {
                    if let Some(message) = msg.text() {
                        match message {
                            m if mention_parameters.rust_regex.is_match(m) => {
                                handle_rust_matched_mention(
                                    bot,
                                    msg,
                                    db_pool,
                                    mention_parameters.req_time_diff,
                                )
                                .await
                            }
                            m if mention_parameters.blazing_fast_regex.is_match(m) => {
                                handle_bf_matched_mention(bot, msg).await
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
        .dependencies(dptree::deps![mention_parameters, db_pool])
        // If the dispatcher fails for some reason, execute this handler.
        .error_handler(LoggingErrorHandler::with_custom_text(
            "An error has occurred in the dispatcher",
        ))
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

async fn handle_bf_matched_mention(bot: Bot, msg: Message) {
    bot.send_message(msg.chat.id, "Did you mean Rust? 👉👈".to_string())
        .reply_to_message_id(msg.id)
        .message_thread_id(msg.thread_id.unwrap_or(0))
        .await
        .map_err(|err| error!("Can't send reply: {:?}", err))
        .ok();
}

async fn handle_rust_matched_mention(
    bot: Bot,
    message: Message,
    db_pool: PgPool,
    req_time_diff: Duration,
) {
    let message_date = message.date.timestamp();
    let curr_native_date = NaiveDateTime::from_timestamp_opt(message_date, 0).unwrap();
    let curr_date: DateTime<Utc> = DateTime::from_utc(curr_native_date, Utc);
    info!("mention time: {}", curr_date);

    if let Common(MessageCommon {
        from:
            Some(User {
                id: user_id,
                username: Some(username),
                ..
            }),
        ..
    }) = message.kind
    {
        // pool the latest mention time from db
        let last_mention_time =
            mention_repository::lead_earliest_mention_time(&db_pool, message.chat.id.0).await;
        let last_update_time = Utc.from_utc_datetime(&last_mention_time);
        info!("latest update time: {}", last_update_time);

        let time_diff = curr_date.signed_duration_since(last_update_time);
        if time_diff > req_time_diff {
            let message_ids = (message.id, message.chat.id, message.thread_id.unwrap_or(0));
            send_rust_mention_response(bot, message_ids, time_diff, &username).await;
        }

        mention_repository::insert_mention(
            &db_pool,
            user_id.0 as i64,
            &username,
            message
                .thread_id
                .map_or_else(|| message.chat.id.0, |id| id as i64),
        )
        .await;
    }
}

async fn send_rust_mention_response(
    bot: Bot,
    message_ids: (MessageId, ChatId, i32),
    time_diff: Duration,
    username: &str,
) {
    bot.send_message(
        message_ids.1,
        format!(
            "Hi, {}! You just wrote smth about Rust! \nBe careful, \
                    {}d:{}h:{}m since last incident.",
            username,
            time_diff.num_days(),
            time_diff.num_hours() % HOURS_PER_DAY,
            time_diff.num_minutes() % MINUTES_PER_HOUR
        ),
    )
    .message_thread_id(message_ids.2)
    .reply_to_message_id(message_ids.0)
    .await
    .map_err(|err| error!("Can't send reply: {:?}", err))
    .ok();

    bot.send_sticker(
        message_ids.1,
        InputFile::file_id(fetch_sticker_id(time_diff)),
    )
    .message_thread_id(message_ids.2)
    .await
    .map_err(|err| error!("Can't send a sticker: {:?}", err))
    .ok();
}

fn fetch_sticker_id(time_diff: Duration) -> &'static str {
    let sticker_index = time_diff.num_minutes().rem_euclid(STICKERS.len() as i64) as usize;
    STICKERS[sticker_index]
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
    req_time_diff: Duration,
}

#[cfg(test)]
mod tests {
    use crate::{fetch_sticker_id, STICKERS};
    use chrono::Duration;

    #[test]
    fn test_fetch_sticker_id() {
        let sticker_id = fetch_sticker_id(Duration::minutes(7));
        assert_eq!(sticker_id, STICKERS[2]);
    }
}
