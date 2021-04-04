use teloxide::prelude::*;
use teloxide::types::InputFile;

static STICKER_ID: &str = "CAACAgEAAxkBAAOrYGoytP93yNKPRS6jo39dCGmuXnUAAlcBAAJpejEFk0uf6g86yKAeBA";

#[tokio::main]
async fn main() {
    run().await;
}

async fn run() {
    teloxide::enable_logging!();
    log::info!("Starting bot...");

    let bot = Bot::from_env().auto_send();

    teloxide::repl(bot, |message| async move {
        message.answer("Hello there").await?;
        message
            .answer_sticker(InputFile::file_id(STICKER_ID))
            .await?;
        respond(())
    })
    .await;
}
