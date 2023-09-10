use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub choices: Vec<ChatCompletionResponseChoice>,
    pub usage: ChatCompletionResponseUsage
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionResponseChoice {
    pub index: i32,
    pub message: ChatCompletionMessage,
    pub finish_reason: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionResponseUsage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: f32
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatCompletionMessage>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub name: Option<String>
}