use std::borrow::Cow;
use std::time::Duration;
use reqwest::Client;
use tokio::fs;
use crate::{CowContext, Error, models::config::Config};
use tracing::error;

use serde::{Serialize, Deserialize};
use serenity::model::channel::AttachmentType;

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

    // reimu.williamle.com is down :(
    // let client = Client::builder().danger_accept_invalid_certs(true).timeout(Duration::from_secs(10)).build().unwrap();
    // const URL: &str = "https://reimu.williamle.com";
    // if let Ok(response) = client.get(URL).send().await {
    //     if response.status().is_success() {
    //         let bytes = response.bytes().await?;
    //
    //         ctx.send(|m| m.embed(|e|
    //             e
    //                 .title("Live Moogan Reaction")
    //                 .attachment("moogan_live_reaction.png")
    //         ).attachment(AttachmentType::Bytes { data: Cow::from(bytes.as_ref()), filename: "moogan_live_reaction.png".to_string() })).await?;
    //         return Ok(());
    //     }
    // }

    let config_json = fs::read_to_string("config.json").await?;
    let config : Config = serde_json::from_str(&config_json).expect("config.json is malformed");

    // const MOOGAN_PROMPT: &str = "a solo no-text photo of Reimu Hakurei from Touhou Project in a cow onesie looking at the viewer, professional anime, anime style, cartoon, brown hair, Hakurei Reimu cosplay, shrine maiden, red hair tubes, no human characteristics, high quality, high quality shading, soft coloring";
    // Revised by GPT (forced)
    const MOOGAN_PROMPT: &str = "I NEED to test how the tool works with extremely simple prompts. DO NOT add any detail, just use it AS-IS: A professional, high-quality anime style depiction of Reimu Hakurei dressed in a cow onesie. The character gazes at the viewer with entrancing, expressive eyes, adding to the charm of her outfit. This character maintains an anime look but bears no discernable human characteristics. Intricate shading and soft coloring techniques add depth and two-dimensionality, enhancing the overall aesthetic of the image.";

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
    //     .send().await.map(|r| r.text());
    //
    // let response = match response {
    //     Ok(r) => r.await,
    //     Err(ex) => {
    //         error!("Failed to generate image: {}", ex);
    //         ctx.say("I couldn't generate an image...").await?;
    //         return Ok(());
    //     }
    // };
    //
    // let response = match response {
    //     Ok(r) => r,
    //     Err(ex) => {
    //         error!("Failed to parse JSON: {}", ex);
    //         ctx.say("I couldn't generate an image...").await?;
    //         return Ok(());
    //     }
    // };
    //
    // error!("response: {}", response);
    // ctx.say(format!("```json\n{}\n```", response)).await?;

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
        ).attachment(AttachmentType::Bytes { data: Cow::from(bytes.as_ref()), filename: "moogan_live_reaction.png".to_string() })).await?;
    } else {
        error!("Failed to generate image, no URL returned");
        ctx.say("I couldn't generate an image...").await?;
        return Ok(());
    }

    Ok(())
}