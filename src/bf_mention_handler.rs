use log::{error, info};
use teloxide::prelude::*;
use teloxide::types::ReplyParameters;

pub async fn handle_bf_matched_mention(bot: Bot, msg: Message) {
    let chat_id = msg.chat.id;
    info!("bf mention invocation: chat_id: {}", chat_id);
    bot.send_message(chat_id, "Did you mean Rust? 👉👈".to_string())
        .reply_parameters(ReplyParameters::new(msg.id))
        .message_thread_id(
            msg.thread_id
                .expect("Couldn't extract thread id from message"),
        )
        .await
        .map_err(|err| error!("Can't send reply: {:?}", err))
        .ok();
}
