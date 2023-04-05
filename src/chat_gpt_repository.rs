use crate::chat_gpt_handler::ChatMessage;
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, RedisResult};

pub async fn get_context(
    connection_manager: &mut ConnectionManager,
    key: &String,
) -> RedisResult<Vec<ChatMessage>> {
    connection_manager.lrange(key, 0, 19).await
}

pub async fn set_context(
    redis_connection_manager: &mut ConnectionManager,
    key: &String,
    context: Vec<&ChatMessage>,
) -> RedisResult<()> {
    redis_connection_manager.lpush(key, context).await
}
