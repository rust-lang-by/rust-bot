#[macro_use]
extern crate lazy_static;

use teloxide::prelude::*;
use teloxide::types::InputFile;
use regex::Regex;

const STICKER_ID: &str = "CAACAgEAAxkBAAOrYGoytP93yNKPRS6jo39dCGmuXnUAAlcBAAJpejEFk0uf6g86yKAeBA";

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

    let bot = Bot::from_env().auto_send();

    teloxide::repl(bot, |message| async move {
        let input_message = message.update.text().unwrap();

        if RE.is_match(input_message) {
            message.answer(
                format!(
                    "Hi, Name! You just wrote smth about Rust! \nBe careful, \
                         X days since last incident."
                )).await?;
        }

        message
            .answer_sticker(InputFile::file_id(STICKER_ID))
            .await?;
        respond(())
    })
        .await;
}

