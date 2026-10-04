use super::config::{DEFAULT_TOP_N, MAX_TOP_N};
use super::{build_digest, send_digest, HnDigestParameters};
use crate::boot::compile_regex;
use crate::{AppError, GptParameters};
use log::{info, warn};
use regex::Regex;
use std::sync::LazyLock;
use teloxide::prelude::*;
use teloxide::types::ChatAction;

// Telegram only recognises `/hn_digest` as a command (no `-` allowed), but the
// hyphenated spelling is accepted as plain text too. `@botname` is appended by
// clients when the command is picked from the menu in groups.
static HN_DIGEST_COMMAND_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(r"(?is)^/hn[_-]digest(?:@\w+)?(?:\s+(.*?))?\s*$"));

const FAILURE_REPLY: &str = "Не получилось собрать HN дайджест, попробуй позже.";

#[derive(Debug, PartialEq)]
enum HnDigestCommand {
    Run(usize),
    Usage,
}

pub fn is_hn_digest_command(text: &str) -> bool {
    parse_command(text).is_some()
}

fn parse_command(text: &str) -> Option<HnDigestCommand> {
    let captures = HN_DIGEST_COMMAND_RE.captures(text.trim())?;
    let Some(arg) = captures
        .get(1)
        .map(|arg| arg.as_str().trim())
        .filter(|arg| !arg.is_empty())
    else {
        return Some(HnDigestCommand::Run(DEFAULT_TOP_N));
    };
    match arg.parse::<usize>() {
        Ok(n) if (1..=MAX_TOP_N).contains(&n) => Some(HnDigestCommand::Run(n)),
        _ => Some(HnDigestCommand::Usage),
    }
}

fn usage_reply() -> String {
    format!("Использование: /hn_digest [1-{MAX_TOP_N}], по умолчанию {DEFAULT_TOP_N}.")
}

/// Runs inline in the dispatcher, so the chat waits for the whole digest
/// (fetch + summary per story) before the bot handles its next message.
pub async fn handle_hn_digest_command(
    bot: Bot,
    msg: Message,
    gpt_parameters: &GptParameters,
    hn_digest_parameters: &HnDigestParameters,
) -> Result<(), AppError> {
    let Some(command) = msg.text().and_then(parse_command) else {
        return Ok(());
    };
    let chat_id = msg.chat.id;
    let top_n = match command {
        HnDigestCommand::Usage => {
            send_digest(&bot, chat_id, Some(msg.id), msg.thread_id, &[usage_reply()]).await?;
            return Ok(());
        }
        HnDigestCommand::Run(top_n) => top_n,
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
    use HnDigestCommand::{Run, Usage};

    #[test]
    fn bare_command_uses_default_count() {
        assert_eq!(parse_command("/hn_digest"), Some(Run(DEFAULT_TOP_N)));
        assert_eq!(parse_command("  /hn_digest  "), Some(Run(DEFAULT_TOP_N)));
    }

    #[test]
    fn explicit_count_is_used() {
        assert_eq!(parse_command("/hn_digest 5"), Some(Run(5)));
        assert_eq!(parse_command("/hn_digest 1"), Some(Run(1)));
        assert_eq!(parse_command("/hn_digest 10"), Some(Run(10)));
    }

    #[test]
    fn spelling_variants_are_accepted() {
        assert_eq!(parse_command("/hn-digest 2"), Some(Run(2)));
        assert_eq!(parse_command("/HN_Digest"), Some(Run(DEFAULT_TOP_N)));
        assert_eq!(parse_command("/hn_digest@rust_by_bot 4"), Some(Run(4)));
        assert_eq!(
            parse_command("/hn_digest@rust_by_bot"),
            Some(Run(DEFAULT_TOP_N))
        );
    }

    #[test]
    fn invalid_count_asks_for_usage() {
        for text in [
            "/hn_digest 0",
            "/hn_digest 11",
            "/hn_digest -1",
            "/hn_digest five",
            "/hn_digest 2 3",
        ] {
            assert_eq!(parse_command(text), Some(Usage), "{text}");
        }
    }

    #[test]
    fn other_text_is_not_the_command() {
        for text in [
            "hn_digest",
            "please run /hn_digest",
            "/hn_digestfoo",
            "/hn",
            "/hn digest",
            "",
        ] {
            assert_eq!(parse_command(text), None, "{text}");
        }
    }

    #[test]
    fn usage_reply_is_html_safe() {
        assert!(!usage_reply().contains(['<', '>', '&']));
    }
}
