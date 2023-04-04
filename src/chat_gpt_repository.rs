use crate::chat_gpt_handler::ChatMessage;
use redis::{aio::MultiplexedConnection, RedisResult};

pub async fn get_context(
    connection: &MultiplexedConnection,
    key: &String,
) -> RedisResult<Vec<ChatMessage>> {
    let mut con = connection.clone();
    redis::cmd("LRANGE")
        .arg(key)
        .arg(-11)
        .arg(0)
        .query_async(&mut con)
        .await
}

pub async fn set_context(
    connection: &MultiplexedConnection,
    key: &String,
    context: Vec<&ChatMessage>,
) -> RedisResult<()> {
    let mut con = connection.clone();
    redis::cmd("LPUSH")
        .arg(key)
        .arg(context)
        .query_async(&mut con)
        .await
}