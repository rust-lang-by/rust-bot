mod mention_repository;

#[macro_use]
extern crate lazy_static;

use chrono::{DateTime, Duration, NaiveDateTime, TimeZone, Utc};
use regex::Regex;
use sqlx::{Error, PgPool, Pool, Postgres};
use std::env;
use teloxide::prelude::*;
use teloxide::types::InputFile;

const STICKER_ID: &str = "CAACAgEAAxkBAAOrYGoytP93yNKPRS6jo39dCGmuXnUAAlcBAAJpejEFk0uf6g86yKAeBA";
const HOURS_PER_DAY: i64 = 24;
const MINUTES_PER_HOUR: i64 = 60;
const MIN_TIME_DIFF: i64 = 30;
const RUST_REGEX: &str = r"\b[RrРр][AaUuАа][CcSsСс][TtТт]\b";

lazy_static! {
    static ref RE: Regex = Regex::new(RUST_REGEX).unwrap();
    static ref REQ_TIME_DIFF: Duration = Duration::seconds(MIN_TIME_DIFF);
    static ref POOL: PgPool = {
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        PgPool::connect_lazy(&database_url).unwrap()
    };
}

#[tokio::main]
async fn main() {
    run().await;
}

async fn run() {
    teloxide::enable_logging!();
    log::info!("Starting bot...");

    let bot = Bot::from_env().auto_send();
    let regex = Regex::new(RUST_REGEX).unwrap();
    let pool = establish_connection();
    let req_time_diff = Duration::seconds(MIN_TIME_DIFF);

    // pool the latest mention time during app initialization
    let last_mention_time = mention_repository::lead_earliest_mention_time(&pool)
        .await
        .unwrap();
    let mut last_update_time = Utc.from_utc_datetime(&last_mention_time);
    log::info!("latest mention time: {}", last_update_time);

    teloxide::repl(bot, move |message| {
        let cloned_pool = pool.clone();
        let cloned_regex = regex.clone();
        let cloned_time_diff = req_time_diff.clone();

        async move {
            let input_message = message.update.text().unwrap();

            if RE.is_match(input_message) {
                let message_date = message.update.date;
                let curr_native_date = NaiveDateTime::from_timestamp(*&message_date as i64, 0);
                log::info!("curr_native_date: {}", curr_native_date);
                let curr_date = DateTime::from_utc(curr_native_date, Utc);
                log::info!("curr_date: {}", curr_date);
                let time_diff = curr_date.signed_duration_since(last_update_time);
                log::info!("time_diff: {}", time_diff);

                if time_diff > *REQ_TIME_DIFF {
                    let user = message.update.from().unwrap();
                    let username = user.username.as_ref().unwrap();

                    message
                        .answer(format!(
                            "Hi, {}! You just wrote smth about Rust! \nBe careful, \
                    {}d:{}h:{}m since last incident.",
                            username,
                            time_diff.num_days(),
                            time_diff.num_hours() % HOURS_PER_DAY,
                            time_diff.num_minutes() % MINUTES_PER_HOUR
                        ))
                        .await?;

                    message
                        .answer_sticker(InputFile::file_id(STICKER_ID))
                        .await?;

                    last_update_time = curr_date;
                    mention_repository::insert_mention(&*POOL, user.id);
                }
            }

            log::info!("last date second: {}", last_update_time);

            respond(())
        }
    })
    .await;
}

pub async fn establish_connection() -> Pool<Postgres> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgPool::connect(&database_url)
        .await
        .expect("Can't establish connection")
}
