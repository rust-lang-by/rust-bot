use chrono::{DateTime, Duration, TimeZone, Utc};
use log::{info, warn};
use sqlx::PgPool;
use teloxide::prelude::*;
use teloxide::types::{InputFile, MessageId, ReplyParameters, ThreadId, User};

use crate::{mention_repository, AppError};

const STICKERS: &[&str; 5] = &[
    "CAACAgEAAx0CTdy33AAD3mQO6sc3rzklybqG4MMI4MLXpXJIAAKCAQACaXoxBT0NGBN6KJNELwQ",
    "CAACAgEAAx0CTdy33AACAQFkF6KoodtDg4KfcPHlUk_7SRFN7QACkQEAAml6MQW86C1JCZcTkS8E",
    "CAACAgEAAx0CTdy33AACAQxkF6Mu7nPaIs9rmMBfXs71BBPxfgACnAEAAml6MQVkU_PxsG8GmS8E",
    "CAACAgEAAx0CTdy33AACARFkF6RG5xa8L2rn6ENe3NsMktY7GgACaQEAAml6MQWdvTv0FuDgLC8E",
    "CAACAgEAAx0CTdy33AACARNkF6SmiAABryW7RcGozvQDCys7JNUAAlcBAAJpejEFk0uf6g86yKAvBA",
];
const HOURS_PER_DAY: i64 = 24;
const MINUTES_PER_HOUR: i64 = 60;

pub async fn handle_rust_matched_mention(
    bot: Bot,
    message: Message,
    db_pool: PgPool,
    req_time_diff: Duration,
    rust_chat_id: i64,
) -> Result<(), AppError> {
    let message_date = message.date.timestamp();
    let Some(curr_date) = DateTime::from_timestamp(message_date, 0) else {
        warn!("skipping rust mention: nonsensical message timestamp {message_date}");
        return Ok(());
    };
    info!(
        "rust mention invocation: chat_id: {}, time: {}",
        message.chat.id, curr_date
    );

    if let Message {
        from:
            Some(User {
                id: user_id,
                username: Some(username),
                ..
            }),
        ..
    } = message
    {
        // pool the latest mention time from db
        let chat_id = message.chat.id;
        if let Ok(last_mention_time) =
            mention_repository::lead_earliest_mention_time(&db_pool, chat_id.0)
                .await
                .inspect_err(|err| warn!("Can't fetch latest mention time: {err:?}"))
        {
            let last_update_time = Utc.from_utc_datetime(&last_mention_time);
            info!("latest update time: {}", last_update_time);

            let time_diff = curr_date.signed_duration_since(last_update_time);
            if time_diff > req_time_diff && chat_id.0 != rust_chat_id {
                let message_ids = (message.id, chat_id, message.thread_id);
                send_rust_mention_response(bot, message_ids, time_diff, &username).await;
            }

            mention_repository::insert_mention(
                &db_pool,
                user_id.0 as i64,
                &username,
                message
                    .thread_id
                    .map_or_else(|| chat_id.0, |id| id.0 .0 as i64),
            )
            .await
            .inspect_err(|err| warn!("Can't insert mention: {err:?}"))
            .ok();
        }
    }
    Ok(())
}

async fn send_rust_mention_response(
    bot: Bot,
    message_ids: (MessageId, ChatId, Option<ThreadId>),
    time_diff: Duration,
    username: &str,
) {
    let reply_msg = bot
        .send_message(
            message_ids.1,
            format!(
                "Hi, {}! You just wrote smth about Rust! \nBe careful, \
                    {}d:{}h:{}m since last incident.",
                username,
                time_diff.num_days(),
                time_diff.num_hours() % HOURS_PER_DAY,
                time_diff.num_minutes() % MINUTES_PER_HOUR
            ),
        )
        .reply_parameters(ReplyParameters::new(message_ids.0));
    if let Some(thread_id) = message_ids.2 {
        // thread reply
        reply_msg
            .message_thread_id(thread_id)
            .await
            .inspect_err(|err| warn!("Can't send reply: {err:?}"))
            .ok();
        bot.send_sticker(
            message_ids.1,
            InputFile::file_id(fetch_sticker_id(time_diff)),
        )
        .message_thread_id(thread_id)
        .await
        .inspect_err(|err| warn!("Can't send a sticker: {err:?}"))
        .ok();
    } else {
        reply_msg
            .await
            .inspect_err(|err| warn!("Can't send reply: {err:?}"))
            .ok();
        bot.send_sticker(
            message_ids.1,
            InputFile::file_id(fetch_sticker_id(time_diff)),
        )
        .await
        .inspect_err(|err| warn!("Can't send a sticker: {err:?}"))
        .ok();
    }
}

pub fn fetch_sticker_id(time_diff: Duration) -> &'static str {
    let sticker_index = time_diff.num_minutes().rem_euclid(STICKERS.len() as i64) as usize;
    STICKERS[sticker_index]
}

#[cfg(test)]
mod tests {
    use crate::rust_mention_handler::{fetch_sticker_id, STICKERS};
    use chrono::Duration;

    #[test]
    fn test_fetch_sticker_id() {
        let sticker_id = fetch_sticker_id(Duration::minutes(7));
        assert_eq!(sticker_id, STICKERS[2]);
    }
}
