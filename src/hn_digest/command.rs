use super::config::{DEFAULT_TOP_N, MAX_TOP_N};
use super::{build_digest, send_html_messages, HnDigestParameters, ReplyTarget};
use crate::{chat_repository, AppError, GptParameters};
use log::{error, info, warn};
use std::future::Future;
use std::pin::pin;
use std::time::Duration;
use teloxide::prelude::*;
use teloxide::types::ChatAction;
use teloxide::utils::command::BotCommands;

const FAILURE_REPLY: &str = "Не получилось собрать HN дайджест, попробуй позже.";
/// Per chat; bounds how often members can trigger 2×N fetch + LLM calls.
const COOLDOWN: Duration = Duration::from_secs(10 * 60);
/// Telegram shows a chat action for ~5s, so it is re-sent while the digest
/// is being built.
const TYPING_REFRESH: Duration = Duration::from_secs(4);

#[derive(BotCommands, Clone, Debug, PartialEq)]
#[command(rename_rule = "lowercase")]
pub enum Command {
    // The count stays a raw string so a bare `/hn` and a bad count can be
    // answered with the default and a usage hint instead of being ignored.
    // (A `///` doc comment here would leak into `Command::descriptions()`.)
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

fn cooldown_reply() -> String {
    format!(
        "HN дайджест можно запрашивать раз в {} минут, попробуй позже.",
        COOLDOWN.as_secs() / 60
    )
}

/// Usage and cooldown replies are sent inline; the digest itself is built in
/// a tracked background task so the chat's other messages keep flowing.
pub async fn handle_command(
    bot: Bot,
    msg: Message,
    command: Command,
    gpt_parameters: &GptParameters,
    hn_digest_parameters: &HnDigestParameters,
) -> Result<(), AppError> {
    let Command::Hn(arg) = command;
    let target = ReplyTarget::reply_to(&msg);
    let Some(top_n) = parse_count(&arg) else {
        send_html_messages(&bot, target, &[usage_reply()]).await?;
        return Ok(());
    };
    if !claim_cooldown(gpt_parameters, target.chat_id).await {
        send_html_messages(&bot, target, &[cooldown_reply()]).await?;
        return Ok(());
    }
    info!("HN digest command: chat_id {}, top {top_n}", target.chat_id);

    let gpt = gpt_parameters.clone();
    let parameters = hn_digest_parameters.clone();
    hn_digest_parameters.command_tasks.spawn(async move {
        let run = run_command(&bot, &gpt, &parameters, target, top_n);
        if parameters.shutdown.run_until_cancelled(run).await.is_none() {
            info!(
                "HN digest command for chat_id {} dropped by shutdown",
                target.chat_id
            );
        }
    });
    Ok(())
}

/// The `NX` key doubles as an in-flight lock, so one chat never builds two
/// digests at once. Fails open: a Redis outage must not disable the command.
async fn claim_cooldown(gpt_parameters: &GptParameters, chat_id: ChatId) -> bool {
    let mut redis = gpt_parameters.redis_connection_manager.clone();
    let key = format!("hn_digest:cooldown:{chat_id}");
    chat_repository::set_if_absent(&mut redis, &key, COOLDOWN)
        .await
        .unwrap_or_else(|err| {
            warn!("HN digest cooldown check failed for chat_id {chat_id}, allowing: {err}");
            true
        })
}

/// Runs off the dispatcher, so errors are logged here rather than returned.
async fn run_command(
    bot: &Bot,
    gpt: &GptParameters,
    parameters: &HnDigestParameters,
    target: ReplyTarget,
    top_n: usize,
) {
    let digest = build_digest(gpt, &parameters.hn_api_base_url, top_n);
    let messages = match with_typing(bot, target, digest).await {
        Ok(messages) => messages,
        Err(err) => {
            warn!(
                "HN digest command failed for chat_id {}: {err}",
                target.chat_id
            );
            vec![FAILURE_REPLY.to_owned()]
        }
    };
    if let Err(err) = send_html_messages(bot, target, &messages).await {
        error!(
            "HN digest command: can't reply in chat_id {}: {err}",
            target.chat_id
        );
    }
}

async fn with_typing<T>(bot: &Bot, target: ReplyTarget, work: impl Future<Output = T>) -> T {
    let mut work = pin!(work);
    let mut refresh = tokio::time::interval(TYPING_REFRESH);
    loop {
        tokio::select! {
            biased;
            output = &mut work => return output,
            _ = refresh.tick() => send_typing(bot, target).await,
        }
    }
}

/// Best-effort: a missing "typing…" indicator is not worth failing over.
async fn send_typing(bot: &Bot, target: ReplyTarget) {
    let mut typing = bot.send_chat_action(target.chat_id, ChatAction::Typing);
    if let Some(thread_id) = target.thread_id {
        typing = typing.message_thread_id(thread_id);
    }
    typing
        .await
        .inspect_err(|err| {
            warn!(
                "Can't send typing action to chat_id {}: {err}",
                target.chat_id
            )
        })
        .ok();
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
    fn description_is_only_the_menu_text() {
        let descriptions = Command::descriptions().to_string();
        assert!(
            descriptions.contains("Hacker News digest: /hn [1-10], 3 by default."),
            "{descriptions}"
        );
        assert!(!descriptions.contains("raw string"), "{descriptions}");
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
    fn replies_are_html_safe() {
        for reply in [usage_reply(), cooldown_reply(), FAILURE_REPLY.to_owned()] {
            assert!(!reply.contains(['<', '>', '&']), "{reply}");
        }
    }
}
