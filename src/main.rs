mod mention_repository;

use chrono::{DateTime, Duration, NaiveDateTime, TimeZone, Utc};
use regex::Regex;
use sqlx::{PgPool, Pool, Postgres};
use std::env;
use teloxide::prelude::*;
use teloxide::types::InputFile;

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
    let regex = Regex::new(RUST_REGEX).unwrap();
    let pool = establish_connection().await;
    let req_time_diff = Duration::minutes(MIN_TIME_DIFF);

    teloxide::repl(bot, move |message| {
        let cloned_pool = pool.clone();
        let cloned_regex = regex.clone();
        let cloned_time_diff = req_time_diff.clone();

        async move {
            let input_message = message.update.text().unwrap_or("");

            if cloned_regex.is_match(input_message) {
                handle_matched_mention(message, cloned_pool, cloned_time_diff).await;
            }
            respond(())
        }
    })
    .await;
}

async fn handle_matched_mention(
    message: UpdateWithCx<AutoSend<Bot>, Message>,
    cloned_pool: PgPool,
    cloned_time_diff: Duration,
) {
    let message_date = message.update.date;
    let curr_native_date = NaiveDateTime::from_timestamp(*&message_date as i64, 0);
    let curr_date: DateTime<Utc> = DateTime::from_utc(curr_native_date, Utc);
    log::info!("mention time: {}", curr_date);

    // pool the latest mention time from db
    let last_mention_time = mention_repository::lead_earliest_mention_time(&cloned_pool)
        .await
        .expect("Can't pool latest mention time");
    let last_update_time = Utc.from_utc_datetime(&last_mention_time);
    log::info!("latest update time: {}", last_update_time);

    let time_diff = curr_date.signed_duration_since(last_update_time);

    if time_diff > cloned_time_diff {
        send_mention_response(message, cloned_pool, time_diff).await;
    }
}

async fn send_mention_response(
    message: UpdateWithCx<AutoSend<Bot>, Message>,
    cloned_pool: PgPool,
    time_diff: Duration,
) {
    let user = message.update.from().expect("Can't identify user");
    let username = user.username.as_ref().unwrap();

    message
        .reply_to(format!(
            "Hi, {}! You just wrote smth about Rust! \nBe careful, \
                    {}d:{}h:{}m since last incident.",
            username,
            time_diff.num_days(),
            time_diff.num_hours() % HOURS_PER_DAY,
            time_diff.num_minutes() % MINUTES_PER_HOUR
        ))
        .await
        .expect("Can't send reply");

    message
        .answer_sticker(InputFile::file_id(STICKER_ID))
        .await
        .expect("Can't send sticker");

    mention_repository::insert_mention(&cloned_pool, user.id)
        .await
        .expect("Can't insert mention");
}

async fn establish_connection() -> Pool<Postgres> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgPool::connect(&database_url)
        .await
        .expect("Can't establish connection")
}
