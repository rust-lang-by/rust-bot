use log::{error, info};
use teloxide::prelude::*;
use teloxide::types::ReplyParameters;

pub async fn handle_bf_matched_mention(bot: Bot, msg: Message) {
    let chat_id = msg.chat.id;
    info!("bf mention invocation: chat_id: {chat_id}");
    let reply_msg = bot
        .send_message(chat_id, "Did you mean Rust? 👉👈".to_string())
        .reply_parameters(ReplyParameters::new(msg.id));
    if let Some(thread_id) = msg.thread_id {
        reply_msg
            .message_thread_id(thread_id)
            .await
            .map_err(|err| error!("Can't send reply: {err:?}"))
            .ok();
    } else {
        reply_msg
            .await
            .map_err(|err| error!("Can't send reply: {err:?}"))
            .ok();
    }
}
