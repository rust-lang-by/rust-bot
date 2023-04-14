use crate::chat_gpt_handler::ChatMessage;
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, RedisResult};
use tokio::io;
use tokio::time::error::Elapsed;
use tokio::time::{timeout, Duration};

const REDIS_TIMEOUT: Duration = Duration::from_secs(1);

pub async fn get_context(
    connection_manager: &mut ConnectionManager,
    key: &String,
) -> RedisResult<Vec<ChatMessage>> {
    timeout_cmd(REDIS_TIMEOUT, connection_manager.lrange(key, 0, 19)).await
}

pub async fn set_context(
    redis_connection_manager: &mut ConnectionManager,
    key: &String,
    context: Vec<&ChatMessage>,
) -> RedisResult<()> {
    timeout_cmd(REDIS_TIMEOUT, redis_connection_manager.lpush(key, context)).await
}

#[inline]
pub async fn timeout_cmd<T>(
    duration: Duration,
    future: redis::RedisFuture<'_, T>,
) -> RedisResult<T> {
    timeout(duration, future)
        .await
        .map_err(redis_error_from_elapsed)
        .and_then(|v| v)
}

#[inline]
pub fn redis_error_from_elapsed(_: Elapsed) -> redis::RedisError {
    redis::RedisError::from(io::Error::from(io::ErrorKind::TimedOut))
}
