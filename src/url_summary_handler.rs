use crate::gpt_service::ChatMessage;
use crate::gpt_service::ChatMessageRole::{System, User};
use crate::{gpt_service, GPTParameters};
use log::{error, info};
use regex::Regex;
use reqwest::Client;
use std::time::Duration;
use teloxide::prelude::*;
use teloxide::types::MediaKind::Text;
use teloxide::types::MessageEntityKind::TextLink;
use teloxide::types::MessageKind::Common;
use teloxide::types::{MediaText, MessageCommon, ReplyParameters};

const ARTICLE_EXTRACTION_TIMEOUT: Duration = Duration::from_secs(30);
const ARTICLE_SUMMARY_SYSTEM_CONTEXT: &str = "Проанализируй статью и дай краткое содержание. Применяй юмор в анализе. Ответ должен быть структурированным, разбитым на пункты и содержать максимум 300 симвалов.";

pub async fn handle_url_summary(
    bot: Bot,
    msg: Message,
    url_regex: Regex,
    gpt_parameters: &mut GPTParameters,
) {
    if let Common(MessageCommon {
        media_kind: Text(media_text),
        ..
    }) = msg.kind
    {
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
            return;
        };

        let content = get_content_call(&url).await.unwrap();
        let clean_content = html2text::from_read(content.as_bytes(), 120).unwrap();
        // Check if the content is long enough to summarize
        if clean_content.len() < 1000 {
            return;
        }
        let summary =
            get_gpt_summary(&gpt_parameters.chat_gpt_api_token, chat_id, clean_content).await;

        let reply_msg = bot
            .send_message(chat_id, format!("TLDR:\n{}", summary))
            .reply_parameters(ReplyParameters::new(msg.id));
        if let Some(thread_id) = msg.thread_id {
            reply_msg
                .message_thread_id(thread_id)
                .await
                .map_err(|err| error!("Can't send reply: {:?}", err))
                .ok();
        } else {
            reply_msg
                .await
                .map_err(|err| error!("Can't send reply: {:?}", err))
                .ok();
        }
    }
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

async fn get_content_call(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::builder().build()?;

    let response = client
        .get(url)
        .timeout(ARTICLE_EXTRACTION_TIMEOUT)
        .send()
        .await?
        .text()
        .await?;
    Ok(response)
}

pub async fn get_gpt_summary(api_key: &String, chat_id: ChatId, message: String) -> String {
    let system_message = ChatMessage {
        role: System,
        content: ARTICLE_SUMMARY_SYSTEM_CONTEXT.to_string(),
    };
    let content_message = ChatMessage {
        role: User,
        content: message,
    };

    let context = Vec::from([system_message, content_message]);
    gpt_service::chat_gpt_call(api_key, chat_id, context)
        .await
        .content
}
