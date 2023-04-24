use chrono::{Duration, Utc};
use log::{error, info};
use teloxide::prelude::*;
use teloxide::types::MessageKind::Common;
use teloxide::types::{ChatPermissions, MessageCommon, User};

pub async fn handle_gayness_mention(bot: Bot, msg: Message) {
    let chat_id = msg.chat.id;
    info!("gayness mention invocation: chat_id: {}", chat_id);
    if let Common(MessageCommon {
        from: Some(User { id: user_id, .. }),
        ..
    }) = msg.kind
    {
        bot.restrict_chat_member(chat_id, user_id, ChatPermissions::empty())
            .until_date(Utc::now() + Duration::hours(6))
            .await
            .map_err(|err| error!("Can't apply restriction: {:?}", err))
            .ok();
        bot.send_message(
            chat_id,
            "Think about your low gayness in 6-hours mute 😒".to_string(),
        )
        .reply_to_message_id(msg.id)
        .message_thread_id(msg.thread_id.unwrap_or(0))
        .await
        .map_err(|err| error!("Can't send reply: {:?}", err))
        .ok();
    }
}
