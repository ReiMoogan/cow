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
pub struct TagAutocomplete {
    pub label: String,
    pub value: String,
    pub post_count: u64,
    pub antecedent: Option<String>
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
    fetch_by_tag(ctx, &combine_tags("hakurei_reimu", &second_tag), second_tag.map(|o| vec![o])).await
}

#[poise::command(
    prefix_command,
    description_localized("en-US", "Get Momiji images."),
    discard_spare_arguments,
    hide_in_help,
    user_cooldown = "2"
)]
pub async fn momiji(ctx: CowContext<'_>, #[rest] second_tag: Option<String>) -> Result<(), Error> {
    fetch_by_tag(ctx, &combine_tags("inubashiri_momiji", &second_tag), second_tag.map(|o| vec![o])).await
}

#[poise::command(
    prefix_command,
    description_localized("en-US", "Get Sanae images."),
    discard_spare_arguments,
    hide_in_help,
    user_cooldown = "2"
)]
pub async fn sanae(ctx: CowContext<'_>, #[rest] second_tag: Option<String>) -> Result<(), Error> {
    fetch_by_tag(ctx, &combine_tags("kochiya_sanae", &second_tag), second_tag.map(|o| vec![o])).await
}

#[poise::command(
    prefix_command,
    description_localized("en-US", "Get Marisa images."),
    discard_spare_arguments,
    hide_in_help,
    user_cooldown = "2"
)]
pub async fn marisa(ctx: CowContext<'_>, #[rest] second_tag: Option<String>) -> Result<(), Error> {
    fetch_by_tag(ctx, &combine_tags("kirisame_marisa", &second_tag), second_tag.map(|o| vec![o])).await
}

#[poise::command(
    prefix_command,
    description_localized("en-US", "Get Reisen images."),
    discard_spare_arguments,
    hide_in_help,
    user_cooldown = "2"
)]
pub async fn reisen(ctx: CowContext<'_>, #[rest] second_tag: Option<String>) -> Result<(), Error> {
    fetch_by_tag(ctx, &combine_tags("reisen_udongein_inaba", &second_tag), second_tag.map(|o| vec![o])).await
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
    let original = search.map(|o| {
        o
            .split('+')
            .take(2)
            .map(|o| o.trim())
            .map(|o| o.to_string())
            .collect::<Vec<String>>()
    });

    let tag_option = original.as_ref().map(|o| {
        o
            .iter()
            .map(|s| {
                convert_to_tag(s) // Trim and lowercase the tag
            })
            .reduce(|a, b| format!("{a}+{b}")) // Combine the tags
            .unwrap()
    });

    if let Some(tag) = tag_option {

        return fetch_by_tag(ctx, &tag, original).await;
    } else {
        ctx.say("You need to pass a valid Danbooru tag to search for.").await?;
    }

    Ok(())
}

fn convert_to_tag(input: &str) -> String {
    let non_tag = Regex::new(r"[^A-Za-z0-9()_.><*:]").unwrap();

    non_tag.replace_all(&input.trim().to_lowercase(), "_").to_string()
}

fn combine_tags(first: &str, second: &Option<String>) -> String {
    if let Some(second) = second {
        format!("{}+{}", first, convert_to_tag(second))
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

async fn fetch_tag_autocomplete(query: &str, client: &reqwest::Client, danbooru_login: &str, danbooru_api_key: &str) -> Result<Vec<TagAutocomplete>, ()> {
    match client
        .get("https://danbooru.donmai.us/autocomplete.json")
        .basic_auth(danbooru_login, Some(danbooru_api_key))
        .header("User-Agent", "Moogan/0.2.23")
        .query(&[("search[query]", query)])
        .query(&[("search[type]", "tag_query")])
        .query(&[("version", 1)])
        .query(&[("limit", 10)])
        .send()
        .await {
        Ok(data) => {
            let text = data.text().await.unwrap();
            if let Ok(ex) = serde_json::from_str::<DanbooruError>(&text) {
                error!("Danbooru returned an error: {} - {}", ex.error, ex.message);
                return Err(());
            }

            match serde_json::from_str::<Vec<TagAutocomplete>>(&text) {
                Ok(tags) => {
                    Ok(tags)
                },
                Err(ex) => {
                    error!("No results found...: {}", ex);
                    Err(())
                }
            }
        },
        Err(ex) => {
            error!("Failed to send request: {}", ex);
            Err(())
        }
    }
}

async fn handle_failure(ctx: CowContext<'_>, original: Option<Vec<String>>, client: &reqwest::Client, danbooru_login: &str, danbooru_api_key: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(original) = original {
        let mut message = MessageBuilder::new();
        message.push("No results found for your query; you probably misspelled something. Did you mean:\n\n");

        for tag in original {
            message.push("Instead of ").push_mono_safe(&tag).push("\n");

            match fetch_tag_autocomplete(&tag, client, danbooru_login, danbooru_api_key).await {
                Ok(tags) => {
                    for matching_tag in tags {
                        message.push(format!("- `{}`\n", matching_tag.value));
                    }
                },
                Err(_) => {
                    message.push("- Error loading tags.\n");
                }
            }

            message.push("\n");
        }

        ctx.say(message.build()).await?;
    }
    else {
        ctx.say("No results were found, but it doesn't seem to be your fault. Try again later?").await?;
    }

    Ok(())
}

async fn fetch_by_tag(ctx: CowContext<'_>, tag: &str, original: Option<Vec<String>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();

    let config_json = fs::read_to_string("config.json").await?;
    let config : Config = serde_json::from_str(&config_json).expect("config.json is malformed");

    if config.danbooru_login.is_empty() || config.danbooru_api_key.is_empty() {
        error!("Danbooru login or API key is not set in config.json");
        return Ok(());
    }

    let url = if let Ok(channel) = ctx.channel_id().to_channel(ctx).await {
        if channel.is_nsfw() {
            // I'm not even going to test this.
            format!("https://danbooru.donmai.us/posts/random.json?tags={tag}")
        } else {
            format!("https://safebooru.donmai.us/posts/random.json?tags={tag}")
        }
    } else {
        format!("https://safebooru.donmai.us/posts/random.json?tags={tag}")
    };

    match client
        .get(&url)
        .basic_auth(&config.danbooru_login, Some(&config.danbooru_api_key))
        .header("User-Agent", "Moogan/0.2.23")
        .send()
        .await {
        Ok(data) => {
            let text = data.text().await.unwrap();
            if let Ok(ex) = serde_json::from_str::<DanbooruError>(&text) {
                error!("Danbooru returned an error: {} - {}", ex.error, ex.message);
                return handle_failure(ctx, original, &client, &config.danbooru_login, &config.danbooru_api_key).await;
            }

            error!("Response: {}", text);

            match serde_json::from_str::<Post>(&text) {
                Ok(mut post) => {
                    const MAX_ATTEMPTS: u8 = 5;
                    let mut attempts = 0;
                    while !is_nice_post(&post) && attempts < MAX_ATTEMPTS {
                        error!("{}", serde_json::to_string_pretty(&post).unwrap());
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        post = client.get(&url).basic_auth(&config.danbooru_login, Some(&config.danbooru_api_key)).header("User-Agent", "Moogan/0.2.23").send().await.unwrap().json::<Post>().await.unwrap();
                        attempts += 1;
                    }

                    if attempts >= 5 {
                        ctx.say(format!("Failed to get a quality image within {MAX_ATTEMPTS} attempts. Please try again.")).await?;
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

