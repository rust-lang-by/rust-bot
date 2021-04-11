#[macro_use]
extern crate lazy_static;

use chrono::{DateTime, NaiveDateTime, Utc};
use regex::Regex;
use teloxide::prelude::*;
use teloxide::types::InputFile;

const STICKER_ID: &str = "CAACAgEAAxkBAAOrYGoytP93yNKPRS6jo39dCGmuXnUAAlcBAAJpejEFk0uf6g86yKAeBA";
const HOURS_PER_DAY: i64 = 24;
const MINUTES_PER_HOUR: i64 = 60;
const MIN_TIME_DIFF: i64 = 30;

lazy_static! {
    static ref RE: Regex = Regex::new(r"\b[RrРр][AaUuАа][CcSsСс][TtТт]\b").unwrap();
}

#[tokio::main]
async fn main() {
    run().await;
}

async fn run() {
    teloxide::enable_logging!();
    log::info!("Starting bot...");

    let mut last_date: DateTime<Utc> = Utc::now();
    log::info!("last date: {}", last_date);

    let bot = Bot::from_env().auto_send();

    teloxide::repl(bot, |message| async move {
        let input_message = message.update.text().unwrap();

        if RE.is_match(input_message) {
            let message_date = message.update.date;
            let curr_native_date = NaiveDateTime::from_timestamp(*&message_date as i64, 0);
            let curr_date: DateTime<Utc> = DateTime::from_utc(curr_native_date, Utc);
            let time_diff = curr_date.signed_duration_since(last_date);
            let username = message.update.from().unwrap().username.as_ref();

            message
                .answer(format!(
                    "Hi, {}! You just wrote smth about Rust! \nBe careful, \
                    {}d:{}h:{}m since last incident.",
                    username.unwrap(),
                    time_diff.num_days(),
                    time_diff.num_hours() % HOURS_PER_DAY,
                    time_diff.num_minutes() % MINUTES_PER_HOUR
                ))
                .await?;
        }

        message
            .answer_sticker(InputFile::file_id(STICKER_ID))
            .await?;
        respond(())
    })
    .await;
}
