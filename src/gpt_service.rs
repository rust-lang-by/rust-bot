use log::{error, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use teloxide::types::ChatId;

const GPT_REQUEST_TIMEOUT: Duration = Duration::from_secs(90);
const OPEN_AI_COMPLETION_URL: &str = "https://api.openai.com/v1/chat/completions";

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
    api_key: &String,
    chat_id: ChatId,
    messages: Vec<ChatMessage>,
) -> ChatMessage {
    match gpt_call(api_key, chat_id, messages).await {
        Ok(choices) => choices[0].message.to_owned(),
        Err(err) => {
            error!("Can't execute chat_gpt_call: {}", err);
            ChatMessage {
                role: ChatMessageRole::Assistant,
                content: "Братан, давай папазжей, занят сейчас.".to_owned(),
            }
        }
    }
}

async fn gpt_call(
    api_key: &String,
    chat_id: ChatId,
    messages: Vec<ChatMessage>,
) -> Result<Vec<Choice>, Box<dyn std::error::Error>> {
    info!(
        "gpt call invocation from chat_id: {} with context: {:#?}",
        chat_id, messages
    );
    let client = Client::builder().build()?;
    let chat_request = ChatRequest {
        messages,
        model: "gpt-5-mini",
        max_tokens: 1000,
    };
    let response = client
        .post(OPEN_AI_COMPLETION_URL)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&chat_request)
        .timeout(GPT_REQUEST_TIMEOUT)
        .send()
        .await?
        .json::<ChatResponse>()
        .await?;
    info!("gpt call invocation for chat_id {} completed", chat_id);
    Ok(response.choices)
}
