use log::{error, info};
use teloxide::types::{Message, User};

use crate::{chat_repository, GPTParameters};

pub async fn handle_default(msg: Message, mut gpt_parameters: GPTParameters) {
    let chat_id = msg.chat.id;
    info!("default mention handler: chat_id: {}", chat_id);
    if let Message {
        from: Some(User { username: name, .. }),
        ..
    } = &msg
    {
        let usr_msg = format!(
            "{:?} mentioned: {:?};",
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
