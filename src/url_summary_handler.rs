use crate::article_summary::{self, ArticleSummaryError};
use crate::{gpt_service, AppError, GptParameters};
use log::{error, info, warn};
use regex::Regex;
use teloxide::prelude::*;
use teloxide::types::MediaKind::Text;
use teloxide::types::MessageEntityKind::TextLink;
use teloxide::types::MessageKind::Common;
use teloxide::types::{MediaText, MessageCommon, ReplyParameters};

pub async fn handle_url_summary(
    bot: Bot,
    msg: Message,
    url_regex: Regex,
    gpt_parameters: &GptParameters,
) -> Result<(), AppError> {
    let Common(MessageCommon {
        media_kind: Text(media_text),
        ..
    }) = msg.kind
    else {
        return Ok(());
    };

    let msg_text = &media_text.text;
    let chat_id = msg.chat.id;
    info!(
        "url summary invocation: chat_id: {}, msg {}",
        chat_id, msg_text
    );
    let url = url_regex
        .find(msg_text)
        .map(|m| m.as_str().to_string())
        .or(find_link(&media_text));
    let Some(url) = url else {
        info!("No URL found in message: {}", msg_text);
        return Ok(());
    };

    let summary = match article_summary::summarize_article(gpt_parameters, &url).await {
        Ok(summary) => summary,
        Err(err @ (ArticleSummaryError::TooShort(_) | ArticleSummaryError::NotHtml(_))) => {
            info!("Skipping url summary for chat_id {chat_id}: {err}");
            return Ok(());
        }
        Err(ArticleSummaryError::Llm(err)) => {
            error!("Can't summarize article for chat_id {chat_id}: {err}");
            gpt_service::busy_fallback().content
        }
        Err(ArticleSummaryError::Fetch(err)) => return Err(err.into()),
        Err(ArticleSummaryError::Parse(err)) => {
            return Err(AppError::BadInput(format!(
                "failed to parse article HTML: {err}"
            )))
        }
    };

    let reply_msg = bot
        .send_message(chat_id, format!("TLDR:\n{}", summary))
        .reply_parameters(ReplyParameters::new(msg.id));
    if let Some(thread_id) = msg.thread_id {
        reply_msg
            .message_thread_id(thread_id)
            .await
            .inspect_err(|err| warn!("Can't send reply: {err:?}"))
            .ok();
    } else {
        reply_msg
            .await
            .inspect_err(|err| warn!("Can't send reply: {err:?}"))
            .ok();
    }
    Ok(())
}

fn find_link(media_text: &MediaText) -> Option<String> {
    media_text
        .entities
        .iter()
        .map(|el| {
            if let TextLink { url: x } = &el.kind {
                Some(x.to_string())
            } else {
                None
            }
        })
        .find_map(|el| el)
}
