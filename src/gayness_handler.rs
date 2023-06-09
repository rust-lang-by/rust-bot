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
        let mute_duration: Duration = calculate_mute_duration(msg.text());
        bot.restrict_chat_member(chat_id, user_id, ChatPermissions::empty())
            .until_date(Utc::now() + mute_duration)
            .await
            .map_err(|err| error!("Can't apply restriction: {:?}", err))
            .ok();
        bot.send_message(
            chat_id,
            format!("Think about your low 🏳️‍🌈 in {:?} minutes mute 😒", mute_duration.num_minutes()),
        )
        .reply_to_message_id(msg.id)
        .message_thread_id(msg.thread_id.unwrap_or(0))
        .await
        .map_err(|err| error!("Can't send reply: {:?}", err))
        .ok();
    }
}

fn calculate_mute_duration(message: Option<&str>) -> Duration {
    match message {
        Some(msg) => {
            match parse_percentage(msg) {
                Some(_x @ 0...1) => Duration::hours(24),
                Some(_x @ 1..=9) => Duration::hours(5),
                Some(_x @ 10..=19) => Duration::hours(3),
                Some(_x @ 20..=29) => Duration::hours(2),
                Some(_x @ 30..=39) => Duration::hours(1),
                _ => Duration::minutes(30),
            }
        }
        None => Duration::hours(24),
    }
}

fn parse_percentage(msg: &str) -> Option<u32> {
    let percentage_index = msg.find('%')?;
    let first_number_index = msg.find(char::is_numeric)?;
    msg[first_number_index..percentage_index].parse::<u32>().ok()
}

#[cfg(test)]
mod tests {
    use crate::gayness_handler::parse_percentage;

    #[test]
    fn test_percentage_parsing() {
        assert_eq!(parse_percentage("I am 97% human!"), Some(97));
        assert_eq!(parse_percentage("I am 8% human!"), Some(8));
        assert_eq!(parse_percentage("foo"), None);
    }
}
