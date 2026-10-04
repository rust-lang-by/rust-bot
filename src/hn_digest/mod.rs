//! Daily Hacker News digest: summarize the top linked stories once and post
//! them to every configured chat.

mod config;
mod hn_api;
mod message;

pub use config::{HnDigestConfig, DEFAULT_HN_API_BASE_URL};

use crate::{article_summary, GptParameters};
use chrono::{DateTime, Days, NaiveTime, Utc};
use log::{error, info, warn};
use message::DigestEntry;
use std::future::Future;
use std::time::Duration;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use tokio::task::JoinHandle;

const PAUSE_BETWEEN_MESSAGES: Duration = Duration::from_secs(1);

pub fn spawn_scheduler(bot: Bot, gpt: GptParameters, config: HnDigestConfig) -> JoinHandle<()> {
    tokio::spawn(async move {
        info!(
            "HN digest enabled: top {} daily at {} UTC to {} chat(s)",
            config.top_n,
            config.time_utc,
            config.chat_ids.len()
        );
        if config.run_on_startup {
            run_digest(&bot, &gpt, &config).await;
        }
        loop {
            let now = Utc::now();
            let next = next_run_after(now, config.time_utc);
            info!("Next HN digest at {next}");
            tokio::time::sleep((next - now).to_std().unwrap_or_default()).await;
            run_digest(&bot, &gpt, &config).await;
        }
    })
}

pub async fn run_digest(bot: &Bot, gpt: &GptParameters, config: &HnDigestConfig) {
    let ids = match hn_api::top_story_ids(&gpt.http_client, &config.hn_api_base_url).await {
        Ok(ids) => ids,
        Err(err) => {
            error!("HN digest: can't fetch top stories, skipping this run: {err}");
            return;
        }
    };
    let entries = collect_entries(&ids, config.top_n, config.top_n * 2, |id| {
        summarize_story(gpt, &config.hn_api_base_url, id)
    })
    .await;
    if entries.is_empty() {
        error!("HN digest: no story could be summarized, nothing posted");
        return;
    }

    let messages: Vec<String> = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| message::format_entry(index + 1, entry))
        .collect();
    for &chat_id in &config.chat_ids {
        post_to_chat(bot, chat_id, &messages).await;
    }
}

/// Firing exactly at `at` counts as done for today, so a run that finishes
/// within the same second never schedules itself twice.
fn next_run_after(now: DateTime<Utc>, at: NaiveTime) -> DateTime<Utc> {
    let today = now.date_naive().and_time(at).and_utc();
    if today > now {
        today
    } else {
        today + Days::new(1)
    }
}

enum Attempt<T> {
    /// Not digest material (no link, job post, missing item); costs nothing.
    Skipped,
    Failed,
    Summarized(T),
}

/// Walk stories in rank order until `top_n` are summarized or `max_attempts`
/// fetch/summary attempts are spent, so a bad day cannot run unbounded.
async fn collect_entries<T, F, Fut>(
    ids: &[u64],
    top_n: usize,
    max_attempts: usize,
    mut attempt: F,
) -> Vec<T>
where
    F: FnMut(u64) -> Fut,
    Fut: Future<Output = Attempt<T>>,
{
    let mut entries = Vec::with_capacity(top_n);
    let mut attempts = 0;
    for &id in ids {
        if entries.len() >= top_n || attempts >= max_attempts {
            break;
        }
        match attempt(id).await {
            Attempt::Skipped => {}
            Attempt::Failed => attempts += 1,
            Attempt::Summarized(entry) => {
                attempts += 1;
                entries.push(entry);
            }
        }
    }
    entries
}

async fn summarize_story(gpt: &GptParameters, base_url: &str, id: u64) -> Attempt<DigestEntry> {
    let item = match hn_api::item(&gpt.http_client, base_url, id).await {
        Ok(Some(item)) => item,
        Ok(None) => return Attempt::Skipped,
        Err(err) => {
            warn!("HN digest: can't fetch item {id}: {err}");
            return Attempt::Failed;
        }
    };
    let Some(story) = item.into_linked_story() else {
        return Attempt::Skipped;
    };
    match article_summary::summarize_article(gpt, &story.url).await {
        Ok(summary) => Attempt::Summarized(DigestEntry { story, summary }),
        Err(err) => {
            warn!("HN digest: skipping item {id} ({}): {err}", story.url);
            Attempt::Failed
        }
    }
}

async fn post_to_chat(bot: &Bot, chat_id: ChatId, messages: &[String]) {
    for (index, message) in messages.iter().enumerate() {
        // Telegram rate-limits bursts into one chat (~20 msg/min in groups).
        if index > 0 {
            tokio::time::sleep(PAUSE_BETWEEN_MESSAGES).await;
        }
        if let Err(err) = bot
            .send_message(chat_id, message)
            .parse_mode(ParseMode::Html)
            .await
        {
            error!("HN digest: can't post to chat {chat_id}, skipping it: {err}");
            return;
        }
    }
    info!(
        "HN digest: posted {} stories to chat {chat_id}",
        messages.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::collections::HashMap;

    fn utc(h: u32, m: u32, s: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 4, h, m, s).unwrap()
    }

    fn at(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    #[test]
    fn next_run_is_today_when_time_is_ahead() {
        assert_eq!(next_run_after(utc(9, 0, 0), at(17, 0)), utc(17, 0, 0));
    }

    #[test]
    fn next_run_is_tomorrow_when_time_has_passed_or_is_now() {
        let tomorrow = Utc.with_ymd_and_hms(2026, 10, 5, 17, 0, 0).unwrap();
        assert_eq!(next_run_after(utc(17, 0, 0), at(17, 0)), tomorrow);
        assert_eq!(next_run_after(utc(17, 0, 1), at(17, 0)), tomorrow);
        assert_eq!(next_run_after(utc(23, 59, 59), at(17, 0)), tomorrow);
    }

    async fn run_collect(
        ids: &[u64],
        top_n: usize,
        outcomes: &[(u64, char)],
    ) -> (Vec<u64>, Vec<u64>) {
        let outcomes: HashMap<u64, char> = outcomes.iter().copied().collect();
        let mut visited = Vec::new();
        let entries = collect_entries(ids, top_n, top_n * 2, |id| {
            visited.push(id);
            let outcome = match outcomes.get(&id) {
                Some('s') => Attempt::Skipped,
                Some('f') => Attempt::Failed,
                _ => Attempt::Summarized(id),
            };
            std::future::ready(outcome)
        })
        .await;
        (entries, visited)
    }

    #[tokio::test]
    async fn collects_top_n_in_rank_order() {
        let (entries, visited) = run_collect(&[1, 2, 3, 4, 5], 3, &[]).await;
        assert_eq!(entries, vec![1, 2, 3]);
        assert_eq!(visited, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn skipped_and_failed_stories_are_replaced_by_next() {
        let (entries, _) = run_collect(&[1, 2, 3, 4, 5], 2, &[(1, 's'), (2, 'f')]).await;
        assert_eq!(entries, vec![3, 4]);
    }

    #[tokio::test]
    async fn skipped_stories_do_not_consume_attempts() {
        let outcomes: Vec<(u64, char)> = (1..=20).map(|id| (id, 's')).collect();
        let (entries, _) = run_collect(&(1..=21).collect::<Vec<_>>(), 1, &outcomes).await;
        assert_eq!(entries, vec![21]);
    }

    #[tokio::test]
    async fn stops_after_max_attempts() {
        let outcomes: Vec<(u64, char)> = (1..=10).map(|id| (id, 'f')).collect();
        let (entries, visited) = run_collect(&(1..=10).collect::<Vec<_>>(), 2, &outcomes).await;
        assert!(entries.is_empty());
        assert_eq!(visited, vec![1, 2, 3, 4]);
    }

    #[tokio::test]
    async fn returns_partial_digest_when_candidates_run_out() {
        let (entries, _) = run_collect(&[1, 2, 3], 3, &[(2, 'f')]).await;
        assert_eq!(entries, vec![1, 3]);
    }
}
