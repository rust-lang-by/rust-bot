use super::config::{DEFAULT_TOP_N, MAX_TOP_N};
use super::{build_digest, send_digest, HnDigestParameters};
use crate::{AppError, GptParameters};
use log::{info, warn};
use teloxide::prelude::*;
use teloxide::types::ChatAction;
use teloxide::utils::command::BotCommands;

const FAILURE_REPLY: &str = "Не получилось собрать HN дайджест, попробуй позже.";

#[derive(BotCommands, Clone, Debug, PartialEq)]
#[command(rename_rule = "lowercase")]
pub enum Command {
    /// The count stays a raw string so a bare `/hn` and a bad count can be
    /// answered with the default and a usage hint instead of being ignored.
    #[command(description = "Hacker News digest: /hn [1-10], 3 by default.")]
    Hn(String),
}

fn parse_count(arg: &str) -> Option<usize> {
    let arg = arg.trim();
    if arg.is_empty() {
        return Some(DEFAULT_TOP_N);
    }
    arg.parse().ok().filter(|n| (1..=MAX_TOP_N).contains(n))
}

fn usage_reply() -> String {
    format!("Использование: /hn [1-{MAX_TOP_N}], по умолчанию {DEFAULT_TOP_N}.")
}

/// Runs inline in the dispatcher, so the chat waits for the whole digest
/// (fetch + summary per story) before the bot handles its next message.
pub async fn handle_command(
    bot: Bot,
    msg: Message,
    command: Command,
    gpt_parameters: &GptParameters,
    hn_digest_parameters: &HnDigestParameters,
) -> Result<(), AppError> {
    let Command::Hn(arg) = command;
    let chat_id = msg.chat.id;
    let Some(top_n) = parse_count(&arg) else {
        send_digest(&bot, chat_id, Some(msg.id), msg.thread_id, &[usage_reply()]).await?;
        return Ok(());
    };
    info!("HN digest command: chat_id {chat_id}, top {top_n}");

    let mut typing = bot.send_chat_action(chat_id, ChatAction::Typing);
    if let Some(thread_id) = msg.thread_id {
        typing = typing.message_thread_id(thread_id);
    }
    typing
        .await
        .inspect_err(|err| warn!("Can't send typing action to chat_id {chat_id}: {err}"))
        .ok();

    let messages =
        match build_digest(gpt_parameters, &hn_digest_parameters.hn_api_base_url, top_n).await {
            Ok(messages) => messages,
            Err(err) => {
                warn!("HN digest command failed for chat_id {chat_id}: {err}");
                vec![FAILURE_REPLY.to_owned()]
            }
        };
    send_digest(&bot, chat_id, Some(msg.id), msg.thread_id, &messages).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOT: &str = "rust_by_bot";

    fn parse(text: &str) -> Option<Command> {
        Command::parse(text, BOT).ok()
    }

    #[test]
    fn command_is_recognised() {
        assert_eq!(parse("/hn"), Some(Command::Hn(String::new())));
        assert_eq!(parse("/hn 5"), Some(Command::Hn("5".into())));
        assert_eq!(parse("/hn@rust_by_bot 4"), Some(Command::Hn("4".into())));
    }

    #[test]
    fn other_text_is_not_the_command() {
        for text in [
            "hn",
            "please run /hn",
            "/hnx",
            "/hn@other_bot 2",
            "/hn_digest",
            "",
        ] {
            assert_eq!(parse(text), None, "{text}");
        }
    }

    #[test]
    fn missing_count_uses_default() {
        assert_eq!(parse_count(""), Some(DEFAULT_TOP_N));
        assert_eq!(parse_count("  "), Some(DEFAULT_TOP_N));
    }

    #[test]
    fn count_in_range_is_used() {
        assert_eq!(parse_count("1"), Some(1));
        assert_eq!(parse_count(" 5 "), Some(5));
        assert_eq!(parse_count("10"), Some(10));
    }

    #[test]
    fn invalid_count_is_rejected() {
        for arg in ["0", "11", "-1", "five", "2 3"] {
            assert_eq!(parse_count(arg), None, "{arg}");
        }
    }

    #[test]
    fn usage_reply_is_html_safe() {
        assert!(!usage_reply().contains(['<', '>', '&']));
    }
}
