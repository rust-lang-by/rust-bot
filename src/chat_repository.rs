use crate::chat_gpt_handler::ChatMessage;
use log::info;
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, RedisResult};
use tokio::io;
use tokio::time::error::Elapsed;
use tokio::time::{timeout, Duration};

const REDIS_TIMEOUT: Duration = Duration::from_secs(1);

pub async fn get_bot_context(
    connection_manager: &mut ConnectionManager,
    key: &String,
) -> RedisResult<Vec<ChatMessage>> {
    info!("fetching  chat bot context for context_key: {}", key);
    timeout_cmd(REDIS_TIMEOUT, connection_manager.lrange(key, 0, 11)).await
}

pub async fn get_chat_history(
    connection_manager: &mut ConnectionManager,
    key: i64,
) -> RedisResult<Vec<String>> {
    info!("fetching chat history for context_key: {}", key);
    timeout_cmd(REDIS_TIMEOUT, connection_manager.lrange(key, 0, 50)).await
}

pub async fn push_context(
    redis_connection_manager: &mut ConnectionManager,
    key: &String,
    context: Vec<&ChatMessage>,
) -> RedisResult<()> {
    redis_connection_manager.rpush(key, context).await
}

pub async fn push_msg(
    redis_connection_manager: &mut ConnectionManager,
    key: i64,
    msg: String,
) -> RedisResult<()> {
    redis_connection_manager.rpush(key, msg).await
}

#[inline]
async fn timeout_cmd<T>(duration: Duration, future: redis::RedisFuture<'_, T>) -> RedisResult<T> {
    timeout(duration, future)
        .await
        .map_err(redis_error_from_elapsed)
        .and_then(|v| v)
}

#[inline]
fn redis_error_from_elapsed(_: Elapsed) -> redis::RedisError {
    redis::RedisError::from(io::Error::from(io::ErrorKind::TimedOut))
}
