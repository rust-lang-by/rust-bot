//! Daily Hacker News digest: summarize the top linked stories once and post
//! them to every configured chat.

mod command;
mod config;
mod hn_api;
mod message;

pub use command::{handle_command, Command};
pub use config::{HnDigestConfig, DEFAULT_HN_API_BASE_URL};

use crate::{article_summary, GptParameters};
use chrono::{DateTime, Days, NaiveTime, Utc};
use log::{error, info, warn};
use message::DigestEntry;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use teloxide::prelude::*;
use teloxide::types::{MessageId, ParseMode, ReplyParameters, ThreadId};
use teloxide::RequestError;
use thiserror::Error;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

const PAUSE_BETWEEN_MESSAGES: Duration = Duration::from_secs(1);

/// Dispatcher dependency for the `/hn` command; independent of
/// [`HnDigestConfig`] so the command works with the scheduler disabled.
#[derive(Clone)]
pub struct HnDigestParameters {
    pub hn_api_base_url: Arc<str>,
}

impl Default for HnDigestParameters {
    fn default() -> Self {
        Self {
            hn_api_base_url: Arc::from(DEFAULT_HN_API_BASE_URL),
        }
    }
}

/// The task runs until `shutdown` is cancelled; it only ends on its own by
/// panicking, so callers should treat any earlier completion as a failure.
pub fn spawn_scheduler(
    bot: Bot,
    gpt: GptParameters,
    config: HnDigestConfig,
    shutdown: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        info!(
            "HN digest enabled: top {} daily at {} UTC to {} chat(s)",
            config.top_n,
            config.time_utc,
            config.chat_ids.len()
        );
        schedule_daily(config.time_utc, config.run_on_startup, &shutdown, || {
            run_digest(&bot, &gpt, &config)
        })
        .await;
        info!("HN digest scheduler stopped");
    })
}

/// Cancellation also drops an in-flight run, so a shutdown mid-digest may
/// leave some chats with a partial digest; nothing is retried.
async fn schedule_daily<F, Fut>(
    at: NaiveTime,
    run_immediately: bool,
    shutdown: &CancellationToken,
    mut run: F,
) where
    F: FnMut() -> Fut,
    Fut: Future<Output = ()>,
{
    if run_immediately && shutdown.run_until_cancelled(run()).await.is_none() {
        return;
    }
    loop {
        let now = Utc::now();
        let next = next_run_after(now, at);
        info!("Next HN digest at {next}");
        let delay = (next - now).to_std().unwrap_or_default();
        if shutdown
            .run_until_cancelled(tokio::time::sleep(delay))
            .await
            .is_none()
        {
            return;
        }
        if shutdown.run_until_cancelled(run()).await.is_none() {
            return;
        }
    }
}

#[derive(Debug, Error)]
pub enum DigestError {
    #[error("can't fetch top stories: {0}")]
    TopStories(#[from] reqwest::Error),

    #[error("no story could be summarized")]
    NothingSummarized,
}

pub async fn run_digest(bot: &Bot, gpt: &GptParameters, config: &HnDigestConfig) {
    let messages = match build_digest(gpt, &config.hn_api_base_url, config.top_n).await {
        Ok(messages) => messages,
        Err(err) => {
            error!("HN digest: nothing posted this run: {err}");
            return;
        }
    };
    for &chat_id in &config.chat_ids {
        match send_digest(bot, chat_id, None, None, &messages).await {
            Ok(()) => info!(
                "HN digest: posted {} stories to chat {chat_id}",
                messages.len()
            ),
            Err(err) => error!("HN digest: can't post to chat {chat_id}, skipping it: {err}"),
        }
    }
}

/// Summarize the top `top_n` linked stories into ready-to-send HTML messages.
pub async fn build_digest(
    gpt: &GptParameters,
    base_url: &str,
    top_n: usize,
) -> Result<Vec<String>, DigestError> {
    let ids = hn_api::top_story_ids(&gpt.http_client, base_url).await?;
    let entries = collect_entries(&ids, top_n, top_n * 2, |id| {
        summarize_story(gpt, base_url, id)
    })
    .await;
    if entries.is_empty() {
        return Err(DigestError::NothingSummarized);
    }
    Ok(entries
        .iter()
        .enumerate()
        .map(|(index, entry)| message::format_entry(index + 1, entry))
        .collect())
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

/// Stops at the first failed send: a chat that rejects one message (bot
/// removed, no rights) will reject the rest too.
async fn send_digest(
    bot: &Bot,
    chat_id: ChatId,
    reply_to: Option<MessageId>,
    thread_id: Option<ThreadId>,
    messages: &[String],
) -> Result<(), RequestError> {
    for (index, message) in messages.iter().enumerate() {
        // Telegram rate-limits bursts into one chat (~20 msg/min in groups).
        if index > 0 {
            tokio::time::sleep(PAUSE_BETWEEN_MESSAGES).await;
        }
        let mut request = bot
            .send_message(chat_id, message)
            .parse_mode(ParseMode::Html);
        if let Some(reply_to) = reply_to {
            request = request
                .reply_parameters(ReplyParameters::new(reply_to).allow_sending_without_reply());
        }
        if let Some(thread_id) = thread_id {
            request = request.message_thread_id(thread_id);
        }
        request.await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration as StdDuration;

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

    fn hours_from_now(hours: i64) -> NaiveTime {
        (Utc::now() + chrono::Duration::hours(hours)).time()
    }

    fn counting_run(
        runs: &Arc<AtomicUsize>,
        never_finishes: bool,
    ) -> impl FnMut() -> std::pin::Pin<Box<dyn Future<Output = ()> + Send>> {
        let runs = runs.clone();
        move || {
            let runs = runs.clone();
            Box::pin(async move {
                runs.fetch_add(1, Ordering::SeqCst);
                if never_finishes {
                    std::future::pending::<()>().await;
                }
            })
        }
    }

    #[tokio::test]
    async fn schedule_stops_when_cancelled_while_waiting() {
        let shutdown = CancellationToken::new();
        let runs = Arc::new(AtomicUsize::new(0));
        let task = tokio::spawn({
            let shutdown = shutdown.clone();
            let run = counting_run(&runs, false);
            async move { schedule_daily(hours_from_now(6), false, &shutdown, run).await }
        });

        tokio::time::sleep(StdDuration::from_millis(20)).await;
        shutdown.cancel();

        tokio::time::timeout(StdDuration::from_secs(1), task)
            .await
            .expect("scheduler must stop promptly")
            .unwrap();
        assert_eq!(runs.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn schedule_stops_when_cancelled_mid_run() {
        let shutdown = CancellationToken::new();
        let runs = Arc::new(AtomicUsize::new(0));
        let task = tokio::spawn({
            let shutdown = shutdown.clone();
            let run = counting_run(&runs, true);
            async move { schedule_daily(hours_from_now(6), true, &shutdown, run).await }
        });

        tokio::time::sleep(StdDuration::from_millis(20)).await;
        shutdown.cancel();

        tokio::time::timeout(StdDuration::from_secs(1), task)
            .await
            .expect("scheduler must stop promptly")
            .unwrap();
        assert_eq!(runs.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn cancelled_schedule_never_runs() {
        let shutdown = CancellationToken::new();
        shutdown.cancel();
        let runs = Arc::new(AtomicUsize::new(0));

        schedule_daily(
            hours_from_now(6),
            true,
            &shutdown,
            counting_run(&runs, false),
        )
        .await;

        assert_eq!(runs.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn returns_partial_digest_when_candidates_run_out() {
        let (entries, _) = run_collect(&[1, 2, 3], 3, &[(2, 'f')]).await;
        assert_eq!(entries, vec![1, 3]);
    }
}
