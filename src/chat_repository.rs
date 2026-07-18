use crate::chat_gpt_handler::BotProfile;
use crate::gpt_service::ChatMessage;
use log::info;
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, FromRedisValue, RedisResult, RedisWrite, ToRedisArgs, Value};
use tokio::io;
use tokio::time::error::Elapsed;
use tokio::time::{timeout, Duration};

const REDIS_TIMEOUT: Duration = Duration::from_secs(1);

pub async fn get_bot_context(
    connection_manager: &mut ConnectionManager,
    key: &String,
) -> RedisResult<Vec<ChatMessage>> {
    info!("fetching  chat bot context for context_key: {}", key);
    timeout_cmd(connection_manager.lrange(key, 0, 11)).await
}

/// Serialize a value for storage in Redis. Serialization of these plain,
/// owned structs is infallible in practice, and `ToRedisArgs::write_redis_args`
/// has no channel to report an error, so the `expect` is a sanctioned panic
/// site (guarded by the crate-level `expect_used` deny).
#[allow(clippy::expect_used)]
fn to_redis_json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("value must serialize to JSON for Redis")
}

impl ToRedisArgs for ChatMessage {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + RedisWrite,
    {
        out.write_arg_fmt(to_redis_json(self))
    }
}

impl FromRedisValue for ChatMessage {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        let str_value: String = FromRedisValue::from_redis_value(v)?;
        serde_json::from_str(&str_value).map_err(deserialize_error::<Self>)
    }
}

impl ToRedisArgs for BotProfile {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + RedisWrite,
    {
        out.write_arg_fmt(to_redis_json(self))
    }
}

impl FromRedisValue for BotProfile {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        let str_value: String = FromRedisValue::from_redis_value(v)?;
        serde_json::from_str(&str_value).map_err(deserialize_error::<Self>)
    }
}

/// Translate a serde_json failure while reading a value back from Redis into a
/// `RedisError` instead of panicking on malformed/corrupted payloads.
fn deserialize_error<T>(err: serde_json::Error) -> redis::RedisError {
    redis::RedisError::from((
        redis::ErrorKind::TypeError,
        "Can't deserialize value from Redis",
        format!("{}: {err}", std::any::type_name::<T>()),
    ))
}

pub async fn get_chat_history(
    connection_manager: &mut ConnectionManager,
    key: i64,
) -> RedisResult<Vec<String>> {
    info!("fetching chat history for context_key: {}", key);
    timeout_cmd(connection_manager.lrange(key, 0, 20)).await
}

pub async fn push_context(
    redis_connection_manager: &mut ConnectionManager,
    key: &String,
    context: Vec<&ChatMessage>,
) -> RedisResult<()> {
    redis_connection_manager.rpush(key, context).await
}

pub async fn push_bot_msg_identifier(
    redis_connection_manager: &mut ConnectionManager,
    chat_key: &String,
    message_key: i32,
    profile: BotProfile,
) -> RedisResult<()> {
    info!("push bot msg identifier for message_key: {}", message_key);
    redis_connection_manager
        .hset(chat_key, message_key, profile)
        .await
}

pub async fn get_bot_msg_profile(
    redis_connection_manager: &mut ConnectionManager,
    chat_key: &String,
    message_key: i32,
) -> RedisResult<BotProfile> {
    info!("get bot profile for message_key: {}", message_key);
    redis_connection_manager.hget(chat_key, message_key).await
}

#[inline]
async fn timeout_cmd<T>(future: redis::RedisFuture<'_, T>) -> RedisResult<T> {
    timeout(REDIS_TIMEOUT, future)
        .await
        .map_err(redis_error_from_elapsed)
        .and_then(|v| v)
}

#[inline]
fn redis_error_from_elapsed(_: Elapsed) -> redis::RedisError {
    redis::RedisError::from(io::Error::from(io::ErrorKind::TimedOut))
}
