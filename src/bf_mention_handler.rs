use log::{error, info};
use teloxide::prelude::*;

pub async fn handle_bf_matched_mention(bot: Bot, msg: Message) {
    let chat_id = msg.chat.id;
    info!("bf mention invocation: chat_id: {}", chat_id);
    bot.send_message(chat_id, "Did you mean Rust? 👉👈".to_string())
        .reply_to_message_id(msg.id)
        .message_thread_id(msg.thread_id.unwrap_or(0))
        .await
        .map_err(|err| error!("Can't send reply: {:?}", err))
        .ok();
}
