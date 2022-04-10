use std::env;

use chrono::{DateTime, Duration, NaiveDateTime, TimeZone, Utc};
use regex::Regex;
use sqlx::{PgPool, Pool, Postgres};
use teloxide::prelude2::*;
use teloxide::types::MessageKind::Common;
use teloxide::types::{InputFile, MessageCommon, User};

mod mention_repository;

const STICKER_ID: &str = "CAACAgEAAxkBAAOrYGoytP93yNKPRS6jo39dCGmuXnUAAlcBAAJpejEFk0uf6g86yKAeBA";
const HOURS_PER_DAY: i64 = 24;
const MINUTES_PER_HOUR: i64 = 60;
const MIN_TIME_DIFF: i64 = 15;
const RUST_REGEX: &str = r"\b[RrРр][AaUuАа][CcSsСс][TtТт]\b";

#[tokio::main]
async fn main() {
    run().await;
}

async fn run() {
    teloxide::enable_logging!();
    log::info!("Starting bot...");

    let bot = Bot::from_env().auto_send();
    let db_pool = establish_connection().await;
    let mention_parameters = MentionParameters {
        regex: Regex::new(RUST_REGEX).expect("Can't compile regex"),
        req_time_diff: Duration::minutes(MIN_TIME_DIFF),
    };

    let handler = Update::filter_message().branch(
        // Filtering allow you to filter updates by some condition.
        dptree::filter(|msg: Message| true)
            // An endpoint is the last update handler.
            .endpoint(
                |msg: Message,
                 mention_parameters: MentionParameters,
                 db_pool: Pool<Postgres>,
                 bot: AutoSend<Bot>| async move {
                    if let Some(message) = msg.text() {
                        if mention_parameters.regex.is_match(message) {
                            handle_matched_mention(
                                bot,
                                msg,
                                db_pool,
                                mention_parameters.req_time_diff,
                            )
                            .await;
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
        .build()
        .setup_ctrlc_handler()
        .dispatch()
        .await;
}

async fn handle_matched_mention(
    bot: AutoSend<Bot>,
    message: Message,
    db_pool: PgPool,
    req_time_diff: Duration,
) {
    let message_date = message.date.timestamp();
    let curr_native_date = NaiveDateTime::from_timestamp(message_date, 0);
    let curr_date: DateTime<Utc> = DateTime::from_utc(curr_native_date, Utc);
    log::info!("mention time: {}", curr_date);

    // pool the latest mention time from db
    let last_mention_time = mention_repository::lead_earliest_mention_time(&db_pool).await;
    let last_update_time = Utc.from_utc_datetime(&last_mention_time);
    log::info!("latest update time: {}", last_update_time);

    let time_diff = curr_date.signed_duration_since(last_update_time);

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
        if time_diff > req_time_diff {
            send_mention_response(bot, message.chat.id, message.id, time_diff, &username).await;
        }

        mention_repository::insert_mention(&db_pool, user_id, &username).await;
    }
}

async fn send_mention_response(
    bot: AutoSend<Bot>,
    chat_id: i64,
    message_id: i32,
    time_diff: Duration,
    username: &String,
) {
    bot.send_message(
        chat_id,
        format!(
            "Hi, {}! You just wrote smth about Rust! \nBe careful, \
                    {}d:{}h:{}m since last incident.",
            username,
            time_diff.num_days(),
            time_diff.num_hours() % HOURS_PER_DAY,
            time_diff.num_minutes() % MINUTES_PER_HOUR
        ),
    )
    .reply_to_message_id(message_id)
    .await
    .map_err(|err| log::error!("Can't send reply: {:?}", err))
    .ok();

    bot.send_sticker(chat_id, InputFile::file_id(STICKER_ID))
        .await
        .map_err(|err| log::error!("Can't send a sticker: {:?}", err))
        .ok();
}

async fn establish_connection() -> Pool<Postgres> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgPool::connect(&database_url)
        .await
        .expect("Can't establish connection")
}

#[derive(Clone)]
struct MentionParameters {
    regex: Regex,
    req_time_diff: Duration,
}
