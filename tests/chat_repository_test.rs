//! Pure (no-container) coverage for the Redis serde impls in `chat_repository`:
//! the `ToRedisArgs` / `FromRedisValue` round-trip and the error path a
//! malformed/corrupt payload takes (must return a `RedisError`, not panic).

use redis::{FromRedisValue, ToRedisArgs, Value};
use rust_bot::chat_gpt_handler::BotProfile;
use rust_bot::gpt_service::{ChatMessage, ChatMessageRole};

#[test]
fn chat_message_round_trips_through_redis_encoding() {
    let original = ChatMessage {
        role: ChatMessageRole::User,
        content: "привет, как дела?".to_string(),
    };

    let args = original.to_redis_args();
    assert_eq!(args.len(), 1, "ChatMessage should encode to a single arg");

    let decoded = ChatMessage::from_redis_value(&Value::Data(args[0].clone()))
        .expect("decode ChatMessage from its own encoding");

    assert_eq!(decoded.content, original.content);
    // `ChatMessageRole` has no `PartialEq`; compare via its serialized form.
    assert_eq!(
        serde_json::to_string(&decoded.role).unwrap(),
        serde_json::to_string(&original.role).unwrap(),
    );
}

#[test]
fn bot_profile_round_trips_through_redis_encoding() {
    for profile in [BotProfile::Fedor, BotProfile::Felix, BotProfile::Ferris] {
        let args = profile.to_redis_args();
        let decoded = BotProfile::from_redis_value(&Value::Data(args[0].clone()))
            .expect("decode BotProfile from its own encoding");
        assert_eq!(decoded, profile);
    }
}

#[test]
fn chat_message_from_malformed_value_is_error_not_panic() {
    let result = ChatMessage::from_redis_value(&Value::Data(b"not valid json".to_vec()));
    assert!(
        result.is_err(),
        "a corrupt ChatMessage payload must return an error"
    );
}

#[test]
fn bot_profile_from_unknown_variant_is_error_not_panic() {
    let result = BotProfile::from_redis_value(&Value::Data(b"\"NotAProfile\"".to_vec()));
    assert!(
        result.is_err(),
        "an unknown BotProfile must return an error"
    );
}
