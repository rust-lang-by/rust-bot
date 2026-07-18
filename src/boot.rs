use std::sync::Arc;

use chrono::Duration;
use redis::aio::ConnectionManager;
use regex::Regex;
use sqlx::{PgPool, Pool, Postgres};
use teloxide::dispatching::UpdateHandler;
use teloxide::error_handlers::LoggingErrorHandler;
use teloxide::prelude::*;
use teloxide::types::MediaKind::Text;
use teloxide::types::MessageEntityKind::{TextLink, Url};
use teloxide::types::MessageKind::Common;
use teloxide::types::{MediaText, MessageCommon};
use teloxide::RequestError;

use crate::{
    bf_mention_handler, chat_gpt_handler, gayness_handler, rust_mention_handler,
    url_summary_handler,
};

const RUST_REGEX: &str = r"(?i)(rust|раст)(.\W|.$|\W|$)";
const BLAZING_FAST_REGEX: &str = r"\w*[BbБб][LlЛл]\w*\W[FfФф][AaАа]\w*\b";
const GAYNESS_REGEX: &str = r"(\D[0-4]|\D)\d%\Dg";
const CHAT_GPT_REGEX: &str = r"(?i)(fedor|ф[её]дор|федя|felix|феликс|feris|ferris|ферис|феррис)";
const URL_REGEX: &str = r#"https?://[^\s<>"{}|\\^`\[\]]*"#;
const MIN_TIME_DIFF: i64 = 15;

pub const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com/v1/chat/completions";

#[derive(Clone)]
pub struct GptParameters {
    pub chat_gpt_api_token: Arc<str>,
    pub openai_base_url: Arc<str>,
    pub http_client: reqwest::Client,
    pub redis_connection_manager: ConnectionManager,
}

#[derive(Clone)]
pub struct MentionParameters {
    pub rust_regex: Regex,
    pub blazing_fast_regex: Regex,
    pub gayness_regex: Regex,
    pub chat_gpt_regex: Regex,
    pub url_regex: Regex,
    pub req_time_diff: Duration,
}

impl Default for MentionParameters {
    fn default() -> Self {
        Self {
            rust_regex: Regex::new(RUST_REGEX).expect("Can't compile regex"),
            blazing_fast_regex: Regex::new(BLAZING_FAST_REGEX).expect("Can't compile regex"),
            gayness_regex: Regex::new(GAYNESS_REGEX).expect("Can't compile regex"),
            chat_gpt_regex: Regex::new(CHAT_GPT_REGEX).expect("Can't compile regex"),
            url_regex: Regex::new(URL_REGEX).expect("Can't compile regex"),
            req_time_diff: Duration::minutes(MIN_TIME_DIFF),
        }
    }
}

pub struct AppDeps {
    pub bot: Bot,
    pub db_pool: PgPool,
    pub gpt_parameters: GptParameters,
    pub mention_parameters: MentionParameters,
}

pub fn build_handler() -> UpdateHandler<RequestError> {
    Update::filter_message().branch(
        dptree::filter(|msg: Message| !msg.chat.is_private()).endpoint(
            |msg: Message,
             mention_parameters: MentionParameters,
             db_pool: Pool<Postgres>,
             gpt_parameters: GptParameters,
             bot: Bot| async move {
                if let Common(MessageCommon {
                    media_kind: Text(media_text),
                    ..
                }) = &msg.kind
                {
                    match &media_text.text {
                        text if mention_parameters.chat_gpt_regex.is_match(text) => {
                            chat_gpt_handler::handle_chat_gpt_question(bot, msg, &gpt_parameters)
                                .await
                        }
                        text if message_has_url(
                            &mention_parameters.url_regex,
                            text,
                            media_text,
                        ) =>
                        {
                            url_summary_handler::handle_url_summary(
                                bot,
                                msg,
                                mention_parameters.url_regex.clone(),
                                &gpt_parameters,
                            )
                            .await
                        }
                        text if mention_parameters.rust_regex.is_match(text) => {
                            rust_mention_handler::handle_rust_matched_mention(
                                bot,
                                msg,
                                db_pool,
                                mention_parameters.req_time_diff,
                            )
                            .await
                        }
                        text if mention_parameters.blazing_fast_regex.is_match(text) => {
                            bf_mention_handler::handle_bf_matched_mention(bot, msg).await
                        }
                        text if mention_parameters.gayness_regex.is_match(text) => {
                            gayness_handler::handle_gayness_mention(bot, msg).await
                        }
                        _ => {
                            if let Some(reply_msg) = &msg.reply_to_message() {
                                chat_gpt_handler::handle_reply(
                                    &bot,
                                    &msg,
                                    reply_msg,
                                    &gpt_parameters,
                                )
                                .await;
                            }
                        }
                    }
                }

                respond(())
            },
        ),
    )
}

pub async fn run(deps: AppDeps) -> anyhow::Result<()> {
    let AppDeps {
        bot,
        db_pool,
        gpt_parameters,
        mention_parameters,
    } = deps;
    let handler = build_handler();
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![mention_parameters, db_pool, gpt_parameters])
        .error_handler(LoggingErrorHandler::with_custom_text(
            "An error has occurred in the dispatcher",
        ))
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
    Ok(())
}

pub fn message_has_url(regex: &Regex, message_text: &str, text: &MediaText) -> bool {
    let has_url_match = regex.is_match(message_text)
        || text
            .entities
            .iter()
            .any(|entity| matches!(entity.kind, TextLink { .. } | Url));

    // Exclude Instagram and TikTok URLs
    if has_url_match {
        !message_text.contains("instagram.") && !message_text.contains("tiktok.")
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::{message_has_url, CHAT_GPT_REGEX, RUST_REGEX, URL_REGEX};
    use regex::Regex;
    use teloxide::types::MediaText;

    #[test]
    fn test_rust_gpt_regex() {
        let chat_gpt_regex = Regex::new(RUST_REGEX).expect("Can't compile regex");
        assert!(chat_gpt_regex.is_match("test rust test"));
        assert!(chat_gpt_regex.is_match("RusT"));
        assert!(chat_gpt_regex.is_match("что там у раста"));
        assert!(chat_gpt_regex.is_match("чэ тупо раст тэст"));
    }

    #[test]
    fn test_chat_gpt_regex() {
        let chat_gpt_regex = Regex::new(CHAT_GPT_REGEX).expect("Can't compile regex");
        assert!(chat_gpt_regex.is_match("ухх Федор как дела?"));
        assert!(chat_gpt_regex.is_match("pFedor tests"));
        assert!(chat_gpt_regex.is_match("p Felix greate"));
        assert!(chat_gpt_regex.is_match("Феликс"));
        assert!(chat_gpt_regex.is_match("[[[Ferris"));
        assert!(chat_gpt_regex.is_match("[ Фёдор ъ"));
    }

    #[test]
    fn test_url_regex() {
        let url_regex = Regex::new(URL_REGEX).expect("Can't compile regex");
        assert!(url_regex.is_match("https://example.com"));
        assert!(url_regex.is_match("http://test.org"));
        assert!(!url_regex.is_match("not a url"));
    }

    #[test]
    fn test_message_has_url() {
        let url_regex = Regex::new(URL_REGEX).expect("Can't compile regex");
        let message_text = "Check this out: https://example.com".to_string();
        let media_text = MediaText {
            text: message_text.clone(),
            entities: vec![],
            link_preview_options: None,
        };

        assert!(message_has_url(&url_regex, &message_text, &media_text));

        let message_text_no_url = "Just a regular message".to_string();
        assert!(!message_has_url(
            &url_regex,
            &message_text_no_url,
            &media_text
        ));
    }

    #[test]
    fn test_message_has_url_exclude_instagram_tiktok() {
        let url_regex = Regex::new(URL_REGEX).expect("Can't compile regex");
        let message_text = "Check this out: https://instagram.com/example".to_string();
        let media_text = MediaText {
            text: message_text.clone(),
            entities: vec![],
            link_preview_options: None,
        };

        assert!(!message_has_url(&url_regex, &message_text, &media_text));

        let message_text_tiktok = "Check this out: https://tiktok.com/example".to_string();
        assert!(!message_has_url(
            &url_regex,
            &message_text_tiktok,
            &media_text
        ));
    }
}
