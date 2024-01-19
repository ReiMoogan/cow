use std::borrow::Cow;
use std::time::Duration;
use reqwest::Client;
use tokio::fs;
use crate::{CowContext, Error, models::config::Config};
use tracing::error;

use serde::{Serialize, Deserialize};
use serenity::all::CreateAttachment;

#[derive(Debug, Serialize, Deserialize)]
pub struct DallERequest {
    pub model: String,
    pub prompt: String,
    pub n: u8,
    pub size: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DallEResponse {
    pub created: Option<u64>,
    pub data: Option<Vec<Option<DallEImage>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DallEImage {
    pub url: Option<String>
}

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en-US", "Live Moogan reaction."),
    discard_spare_arguments
)]
pub async fn moogan(ctx: CowContext<'_>) -> Result<(), Error> {
    ctx.defer().await?;

    let client = Client::builder().danger_accept_invalid_certs(true).timeout(Duration::from_secs(10)).build().unwrap();
    const URL: &str = "https://reimu.williamle.com";
    if let Ok(response) = client.get(URL).send().await {
        if response.status().is_success() {
            let bytes = response.bytes().await?;

            ctx.send(|m| m.embed(|e|
                e
                    .title("Live Moogan Reaction")
                    .attachment("moogan_live_reaction.png")
            ).attachment(CreateAttachment::bytes(bytes.as_ref(), "moogan_live_reaction.png"))).await?;
            return Ok(());
        }
    }

    let config_json = fs::read_to_string("config.json").await?;
    let config : Config = serde_json::from_str(&config_json).expect("config.json is malformed");

    const MOOGAN_PROMPT: &str = "a solo no-text photo of Reimu Hakurei from Touhou Project in a cow onesie looking at the viewer, anime, anime style, cartoon, brown hair, Hakurei Reimu cosplay, no human characteristics, high quality, high quality shading, soft coloring, adult";

    let body = DallERequest {
        model: "dall-e-3".to_string(),
        prompt: MOOGAN_PROMPT.to_string(),
        n: 1,
        size: "1024x1024".to_string()
    };

    let body_serialized = serde_json::to_string(&body).unwrap();

    let client = Client::new();

    let response = client
        .post("https://api.openai.com/v1/images/generations")
        .header("Content-Type", "application/json")
        .bearer_auth(config.openai_api_key)
        .body(body_serialized)
        .send().await.map(|r| r.json::<DallEResponse>());

    let response = match response {
        Ok(r) => r.await,
        Err(ex) => {
            error!("Failed to generate image: {}", ex);
            ctx.say("I couldn't generate an image...").await?;
            return Ok(());
        }
    };

    let response = match response {
        Ok(r) => r,
        Err(ex) => {
            error!("Failed to parse JSON: {}", ex);
            ctx.say("I couldn't generate an image...").await?;
            return Ok(());
        }
    };

    let url = response.data.and_then(|o| o.first().and_then(|p| p.as_ref().and_then(|q| q.url.clone())));

    if let Some(url) = url {
        // download to file
        let response = client.get(url).send().await?;
        let bytes = response.bytes().await?;

        ctx.send(|m| m.embed(|e|
            e
                .title("Live Moogan Reaction")
                .attachment("moogan_live_reaction.png")
        ).attachment(CreateAttachment::bytes(Cow::from(bytes.as_ref()), "moogan_live_reaction.png"))).await?;
    } else {
        error!("Failed to generate image, no URL returned");
        ctx.say("I couldn't generate an image...").await?;
        return Ok(());
    }

    Ok(())
}
