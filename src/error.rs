use thiserror::Error;

/// Domain error for rust-bot.
///
/// Low-level failures from the infrastructure the bot talks to (Redis,
/// Postgres, the Telegram Bot API, outbound HTTP) are translated into these
/// variants at the handler boundaries via `#[from]`, so handlers can propagate
/// with `?` and the dispatcher can log a single, typed error.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("postgres error: {0}")]
    Postgres(#[from] sqlx::Error),

    #[error("telegram error: {0}")]
    Telegram(#[from] teloxide::RequestError),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("gpt error: {0}")]
    Gpt(String),

    #[error("bad input: {0}")]
    BadInput(String),
}
