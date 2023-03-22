use log::error;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use teloxide::prelude::*;

pub async fn handle_chat_gpt_question(bot: Bot, msg: Message, chat_gpt_api_token: String) {
    let message = msg.text().unwrap();
    let slice = &message[2..message.len()];
    let chat_response = chat_gpt_call(slice, chat_gpt_api_token)
        .await
        .unwrap_or_else(|_| "ChatGPT can't process request".to_string());
    bot.send_message(msg.chat.id, chat_response)
        .reply_to_message_id(msg.id)
        .message_thread_id(msg.thread_id.unwrap_or(0))
        .await
        .map_err(|err| error!("Can't send reply: {:?}", err))
        .ok();
}

#[derive(Debug, Deserialize, Serialize)]
struct ChatRequest<'a> {
    messages: Vec<ChatMessageRequest<'a>>,
    model: &'a str,
    max_tokens: i32,
}

#[derive(Debug, Deserialize, Serialize)]
struct ChatMessageRequest<'a> {
    content: &'a str,
    role: &'a str,
}

#[derive(Debug, Deserialize, Serialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Choice {
    message: ChatMessageResponse,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ChatMessageResponse {
    content: String,
    role: String,
}

async fn chat_gpt_call(
    message: &str,
    api_key: String,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::builder().build()?;
    let url = "https://api.openai.com/v1/chat/completions";
    let chat_request = ChatRequest {
        messages: vec![ChatMessageRequest {
            content: message,
            role: "user",
        }],
        model: "gpt-3.5-turbo",
        max_tokens: 1000,
    };
    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&chat_request)
        .send()
        .await?
        .json::<ChatResponse>()
        .await?;
    let text = response.choices[0].message.content.clone();
    Ok(text)
}
