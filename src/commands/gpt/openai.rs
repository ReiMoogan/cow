use reqwest::Client;
use tokio::fs;
use crate::commands::gpt::openai_models::*;
use crate::models::config::Config;

pub async fn create_chat_completion(request: &ChatCompletionRequest) -> Result<ChatCompletionResponse, reqwest::Error> {
    let config_json = fs::read_to_string("config.json").await.expect("config.json is missing");
    let config : Config = serde_json::from_str(&config_json).expect("config.json is malformed");

    let client = Client::new();

    let body_serialized = serde_json::to_string(&request).unwrap();

    client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Content-Type", "application/json")
        .bearer_auth(config.openai_api_key)
        .body(body_serialized)
        .send().await.map(|r| r.json::<ChatCompletionResponse>())?.await
}