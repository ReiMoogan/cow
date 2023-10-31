use std::collections::HashMap;
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
    pub messages: Vec<ChatCompletionMessage>,
    pub functions: Vec<ChatCompletionFunction>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub function_call: Option<ChatCompletionMessageFunctionCall>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionMessageFunctionCall {
    pub name: String,
    pub arguments: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionFunction {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub description: Option<String>,
    pub parameters: ChatCompletionFunctionParameters
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionFunctionParameters {
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub items: Option<Box<ChatCompletionFunctionParameters>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub properties: Option<HashMap<String, ChatCompletionFunctionParameters>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub required: Option<Vec<String>>
}