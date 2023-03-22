use log::error;
use teloxide::prelude::*;

pub async fn handle_bf_matched_mention(bot: Bot, msg: Message) {
    bot.send_message(msg.chat.id, "Did you mean Rust? 👉👈".to_string())
        .reply_to_message_id(msg.id)
        .message_thread_id(msg.thread_id.unwrap_or(0))
        .await
        .map_err(|err| error!("Can't send reply: {:?}", err))
        .ok();
}
