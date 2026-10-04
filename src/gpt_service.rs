use log::info;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::time::Duration;

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

/// The in-character reply chat handlers send when the LLM is unavailable.
pub fn busy_fallback() -> ChatMessage {
    ChatMessage {
        role: ChatMessageRole::Assistant,
        content: "Братан, давай папазжей, занят сейчас.".to_owned(),
    }
}

/// `requester` identifies the caller in logs (a chat id, an article url, ...).
pub async fn chat_gpt_call(
    params: &GptParameters,
    requester: impl Display,
    messages: Vec<ChatMessage>,
) -> Result<ChatMessage, AppError> {
    gpt_call(params, &requester, messages)
        .await?
        .into_iter()
        .next()
        .map(|choice| choice.message)
        .ok_or_else(|| AppError::Gpt(format!("empty choices in response for {requester}")))
}

async fn gpt_call(
    params: &GptParameters,
    requester: &impl Display,
    messages: Vec<ChatMessage>,
) -> Result<Vec<Choice>, AppError> {
    info!(
        "gpt call invocation from {} with context: {:#?}",
        requester, messages
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
    info!("gpt call invocation for {} completed", requester);
    Ok(response.choices)
}
