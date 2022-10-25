use tracing::error;
// Fun with stupid APIs!
use tokio::fs;
use crate::{Config, CowContext, Error};
use serde::Deserialize;
use regex::Regex;

#[derive(Debug, Deserialize)]
pub struct Post {
    // Bytes.
    pub file_size: Option<u64>,
    // Features of the image
    pub tag_string_general: Option<String>,
    pub tag_string_character: Option<String>,
    pub tag_string_artist: Option<String>,
    pub file_url: Option<String>
}

#[poise::command(
    prefix_command,
    description_localized("en-US", "Get Reimu images."),
    discard_spare_arguments,
    hide_in_help,
    user_cooldown = "2"
)]
pub async fn reimu(ctx: CowContext<'_>) -> Result<(), Error> {
    fetch_by_tag(ctx, "hakurei_reimu").await
}

#[poise::command(
    prefix_command,
    description_localized("en-US", "Get Momiji images."),
    discard_spare_arguments,
    hide_in_help,
    user_cooldown = "2"
)]
pub async fn momiji(ctx: CowContext<'_>) -> Result<(), Error> {
    fetch_by_tag(ctx, "inubashiri_momiji").await
}

#[poise::command(
    prefix_command,
    description_localized("en-US", "Get Sanae images."),
    discard_spare_arguments,
    hide_in_help,
    user_cooldown = "2"
)]
pub async fn sanae(ctx: CowContext<'_>) -> Result<(), Error> {
    fetch_by_tag(ctx, "kochiya_sanae").await
}

#[poise::command(
    prefix_command,
    description_localized("en-US", "Get Marisa images."),
    discard_spare_arguments,
    hide_in_help,
    user_cooldown = "2"
)]
pub async fn marisa(ctx: CowContext<'_>) -> Result<(), Error> {
    fetch_by_tag(ctx, "kirisame_marisa").await
}

#[poise::command(
    prefix_command,
    description_localized("en-US", "Get Reisen images."),
    discard_spare_arguments,
    hide_in_help,
    user_cooldown = "2"
)]
pub async fn reisen(ctx: CowContext<'_>) -> Result<(), Error> {
    fetch_by_tag(ctx, "reisen_udongein_inaba").await
}

#[poise::command(
    prefix_command,
    description_localized("en-US", "Find images on Danbooru."),
    discard_spare_arguments,
    hide_in_help,
    user_cooldown = "2"
)]
pub async fn danbooru(
    ctx: CowContext<'_>,
    #[description = "The command requested for help"]
    #[rest] search: Option<String>)
-> Result<(), Error> {
    let non_tag = Regex::new(r"[^A-Za-z0-9()_.><*]").unwrap();
    let tag_option = search
        .map(|o| o.trim().to_lowercase())
        .map(|o| non_tag.replace_all(&*o, "_").to_string());

    if let Some(tag) = tag_option {
        return fetch_by_tag(ctx, &tag).await;
    } else {
        ctx.say("You need to pass a valid Danbooru tag to search for.").await?;
    }

    Ok(())
}

fn is_nice_post(post: &Post) -> bool {
    if post.tag_string_general.is_none() || post.file_url.is_none() || post.file_size.is_none() || post.tag_string_character.is_none() || post.tag_string_artist.is_none() {
        return false;
    }

    let is_comic = post.tag_string_general.clone().unwrap().split(' ').any(|o| o == "comic");
    let character_count = post.tag_string_character.clone().unwrap().split(' ').count();

    post.file_size.unwrap() <= 8 * 1024 * 1024 &&
        character_count <= 3 &&
        !is_comic
}

async fn fetch_by_tag(ctx: CowContext<'_>, tag: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();

    let config_json = fs::read_to_string("config.json").await?;
    let config : Config = serde_json::from_str(&config_json).expect("config.json is malformed");

    if config.danbooru_login.is_empty() || config.danbooru_api_key.is_empty() {
        error!("Danbooru login or API key is not set in config.json");
        return Ok(());
    }

    let url = if let Ok(channel) = ctx.channel_id().to_channel(ctx.discord()).await {
        if channel.is_nsfw() {
            // I'm not even going to test this.
            format!("https://danbooru.donmai.us/posts/random.json?tags={}", tag)
        } else {
            format!("https://danbooru.donmai.us/posts/random.json?tags=rating:s+{}", tag)
        }
    } else {
        format!("https://danbooru.donmai.us/posts/random.json?tags=rating:s+{}", tag)
    };

    match client
        .get(&url)
        .basic_auth(&config.danbooru_login, Some(&config.danbooru_api_key))
        .send()
        .await {
        Ok(data) => {
            let text = data.text().await.unwrap();
            error!("Response: {}", text);
            match serde_json::from_str::<Post>(&*text) {
                Ok(mut post) => {
                    let mut attempts = 0;
                    while !is_nice_post(&post) && attempts < 3 {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        post = client.get(&url).basic_auth(&config.danbooru_login, Some(&config.danbooru_api_key)).send().await.unwrap().json::<Post>().await.unwrap();
                        attempts += 1;
                    }

                    if attempts >= 3 {
                        ctx.say("Temporary failure; rate limit?").await?;
                        return Ok(());
                    }

                    let _ = ctx.send(|m|
                        m.embed(|e|
                            e.title(format!("Artist: {}", post.tag_string_artist.clone().unwrap()))
                                .url(post.file_url.clone().unwrap())
                                //.attachment(file_name);
                                .image(post.file_url.unwrap())
                        )
                    ).await;
                },
                Err(ex) => {
                    error!("No results found...: {}", ex);
                    ctx.say("No results found...").await?;
                }
            }
        },
        Err(ex) => {
            error!("Failed to send request: {}", ex);
            ctx.say("Failed to access Danbooru... try again later?").await?;
        }
    }

    Ok(())
}