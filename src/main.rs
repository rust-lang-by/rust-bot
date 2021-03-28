use teloxide::prelude::*;

#[tokio::main]
async fn main() {
    run().await;
}

async fn run() {
    teloxide::enable_logging!();
    log::info!("Starting bot...");

    let bot = Bot::from_env().send();

    teloxide::repl(bot, |message| async move {
        message.answer_dice().await?;
        respond(())
    })
        .await;
}