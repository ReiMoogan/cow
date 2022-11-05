use tracing::error;
// Fun with stupid APIs!
use tokio::fs;
use crate::{Config, CowContext, Error};
use serde::{Serialize, Deserialize};
use regex::Regex;
use serenity::utils::MessageBuilder;

#[derive(Debug, Serialize, Deserialize)]
pub struct Post {
    // Bytes.
    pub file_size: Option<u64>,
    // Features of the image
    pub tag_string_general: Option<String>,
    pub tag_string_character: Option<String>,
    pub tag_string_artist: Option<String>,
    pub file_url: Option<String>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DanbooruError {
    pub success: bool,
    pub error: String,
    pub message: String
}

#[poise::command(
    prefix_command,
    description_localized("en-US", "Get Reimu images."),
    discard_spare_arguments,
    hide_in_help,
    user_cooldown = "2"
)]
pub async fn reimu(ctx: CowContext<'_>, #[rest] second_tag: Option<String>) -> Result<(), Error> {
    fetch_by_tag(ctx, &*combine_tags("hakurei_reimu", second_tag)).await
}

#[poise::command(
    prefix_command,
    description_localized("en-US", "Get Momiji images."),
    discard_spare_arguments,
    hide_in_help,
    user_cooldown = "2"
)]
pub async fn momiji(ctx: CowContext<'_>, #[rest] second_tag: Option<String>) -> Result<(), Error> {
    fetch_by_tag(ctx, &*combine_tags("inubashiri_momiji", second_tag)).await
}

#[poise::command(
    prefix_command,
    description_localized("en-US", "Get Sanae images."),
    discard_spare_arguments,
    hide_in_help,
    user_cooldown = "2"
)]
pub async fn sanae(ctx: CowContext<'_>, #[rest] second_tag: Option<String>) -> Result<(), Error> {
    fetch_by_tag(ctx, &*combine_tags("kochiya_sanae", second_tag)).await
}

#[poise::command(
    prefix_command,
    description_localized("en-US", "Get Marisa images."),
    discard_spare_arguments,
    hide_in_help,
    user_cooldown = "2"
)]
pub async fn marisa(ctx: CowContext<'_>, #[rest] second_tag: Option<String>) -> Result<(), Error> {
    fetch_by_tag(ctx, &*combine_tags("kirisame_marisa", second_tag)).await
}

#[poise::command(
    prefix_command,
    description_localized("en-US", "Get Reisen images."),
    discard_spare_arguments,
    hide_in_help,
    user_cooldown = "2"
)]
pub async fn reisen(ctx: CowContext<'_>, #[rest] second_tag: Option<String>) -> Result<(), Error> {
    fetch_by_tag(ctx, &*combine_tags("reisen_udongein_inaba", second_tag)).await
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
    let tag_option = validate_tag(search);

    if let Some(tag) = tag_option {
        return fetch_by_tag(ctx, &tag).await;
    } else {
        ctx.say("You need to pass a valid Danbooru tag to search for.").await?;
    }

    Ok(())
}

fn validate_tag(search: Option<String>) -> Option<String> {
    let non_tag = Regex::new(r"[^A-Za-z0-9()_.><*]").unwrap();

    search.map(|o| {
        o
            .split('+') // User can split tags by +
            .take(2) // Only two tags can be searched at a time
            .map(|s| {
                non_tag.replace_all(&*s.trim().to_lowercase(), "_").to_string() // Trim and lowercase the tag
            })
            .reduce(|a, b| format!("{}+{}", a, b)) // Combine the tags
            .unwrap()
    })
}

fn combine_tags(first: &str, second: Option<String>) -> String {
    if let Some(second) = second {
        format!("{}+{}", first, second)
    }
    else {
        first.to_string()
    }
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
            format!("https://safebooru.donmai.us/posts/random.json?tags={}", tag)
        }
    } else {
        format!("https://safebooru.donmai.us/posts/random.json?tags={}", tag)
    };

    match client
        .get(&url)
        .basic_auth(&config.danbooru_login, Some(&config.danbooru_api_key))
        .send()
        .await {
        Ok(data) => {
            let text = data.text().await.unwrap();
            if let Ok(ex) = serde_json::from_str::<DanbooruError>(&*text) {
                error!("Danbooru returned an error: {} - {}", ex.error, ex.message);
                ctx.say("Danbooru returned an error; invalid tag(s)?").await?;
                return Ok(());
            }

            match serde_json::from_str::<Post>(&*text) {
                Ok(mut post) => {
                    const MAX_ATTEMPTS: u8 = 5;
                    let mut attempts = 0;
                    while !is_nice_post(&post) && attempts < MAX_ATTEMPTS {
                        error!("{}", serde_json::to_string_pretty(&post).unwrap());
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        post = client.get(&url).basic_auth(&config.danbooru_login, Some(&config.danbooru_api_key)).send().await.unwrap().json::<Post>().await.unwrap();
                        attempts += 1;
                    }

                    if attempts >= 5 {
                        ctx.say(format!("Failed to get a quality image within {} attempts. Please try again.", MAX_ATTEMPTS)).await?;
                        return Ok(());
                    }

                    let title = MessageBuilder::new()
                        .push("Artist: ")
                        .push_safe(post.tag_string_artist.clone().unwrap_or_else(|| "<unknown>".to_string()))
                        .build();

                    let _ = ctx.send(|m|
                        m.embed(|e|
                            e.title(title)
                                .url(post.file_url.clone().unwrap())
                                //.attachment(file_name);
                                .image(post.file_url.unwrap())
                        )
                    ).await;
                },
                Err(ex) => {
                    error!("No results found...: {}", ex);
                    ctx.say("Danbooru did not provide a valid response...").await?;
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