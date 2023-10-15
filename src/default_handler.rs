use log::{error, info};
use teloxide::types::MessageKind::Common;
use teloxide::types::{Message, MessageCommon, User};

use crate::{chat_repository, GPTParameters};

pub async fn handle_default(msg: Message, mut gpt_parameters: GPTParameters) {
    let chat_id = msg.chat.id;
    info!("gayness mention invocation: chat_id: {}", chat_id);
    if let Common(MessageCommon {
        from: Some(User { username: name, .. }),
        ..
    }) = &msg.kind
    {
        let usr_msg = format!(
            "user:{:?} mentioned:'{:?}'",
            name.clone().unwrap_or_default(),
            &msg.text().unwrap_or_default()
        );
        chat_repository::push_msg(
            &mut gpt_parameters.redis_connection_manager,
            chat_id.0,
            usr_msg,
        )
        .await
        .map_err(|err| error!("Can't update context in Redis: {:?}", err))
        .ok();
    }
}
