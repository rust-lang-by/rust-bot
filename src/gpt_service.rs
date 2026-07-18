use log::{error, info};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use teloxide::types::ChatId;

use crate::{AppError, GptParameters};

const GPT_REQUEST_TIMEOUT: Duration = Duration::from_secs(90);

#[derive(Debug, Deserialize, Serialize)]
struct ChatRequest<'a> {
    messages: Vec<ChatMessage>,
    model: &'a str,
    max_tokens: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatResponse {
    pub choices: Vec<Choice>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Choice {
    pub message: ChatMessage,
}

#[derive(Debug, Deserialize, Serialize, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ChatMessageRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChatMessage {
    pub role: ChatMessageRole,
    pub content: String,
}

pub async fn chat_gpt_call(
    params: &GptParameters,
    chat_id: ChatId,
    messages: Vec<ChatMessage>,
) -> ChatMessage {
    let fallback = || ChatMessage {
        role: ChatMessageRole::Assistant,
        content: "Братан, давай папазжей, занят сейчас.".to_owned(),
    };
    match gpt_call(params, chat_id, messages).await {
        Ok(choices) => choices
            .into_iter()
            .next()
            .map_or_else(fallback, |choice| choice.message),
        Err(err) => {
            error!("Can't execute chat_gpt_call: {}", err);
            fallback()
        }
    }
}

async fn gpt_call(
    params: &GptParameters,
    chat_id: ChatId,
    messages: Vec<ChatMessage>,
) -> Result<Vec<Choice>, AppError> {
    info!(
        "gpt call invocation from chat_id: {} with context: {:#?}",
        chat_id, messages
    );
    let chat_request = ChatRequest {
        messages,
        model: "gpt-4o",
        max_tokens: 1000,
    };
    let response = params
        .http_client
        .post(params.openai_base_url.as_ref())
        .header("Content-Type", "application/json")
        .header(
            "Authorization",
            format!("Bearer {}", params.chat_gpt_api_token.as_ref()),
        )
        .json(&chat_request)
        .timeout(GPT_REQUEST_TIMEOUT)
        .send()
        .await?
        .json::<ChatResponse>()
        .await?;
    info!("gpt call invocation for chat_id {} completed", chat_id);
    Ok(response.choices)
}
