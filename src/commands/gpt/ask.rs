use tokio::fs;
use crate::{CowContext, db, cowdb, Database, Error, models::config::Config};
use chatgpt::client::ChatGPT;
use chatgpt::converse::Conversation;
use tracing::error;
use rand::Rng;
use serenity::utils::MessageBuilder;

const CONVERSATION_PATH: &str = "gpt";

fn new_conversation(client: ChatGPT, ctx: CowContext<'_>) -> Conversation {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let mut rng = rand::thread_rng();
    let ip_first: i32 = rng.gen_range(0..=255);
    let ip_second: i32 = rng.gen_range(0..=255);
    let username = &ctx.author().name;
    client.new_conversation_directed(
        format!("You are MooganGPT, a large language model trained by OpenAI. Answer as concisely as possible. The user you are talking to has an IP address of 169.236.{ip_first}.{ip_second}, and lives at 308 Negra Arroyo Lane, Albuquerque, NM 87105. Their username is {username}, and you are both communicating in a Discord channel. Knowledge cutoff: 2021-09 Current date: {today}")
    )
}

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en-US", "Ask a GPT question without any context."),
    discard_spare_arguments
)]
pub async fn ask(ctx: CowContext<'_>, #[rest] question: Option<String>) -> Result<(), Error> {
    let db = cowdb!(ctx);

    if !db.has_gpt_enabled(ctx.author().id).await? {
        ctx.send(|m| m.content("You are not allowed to use this command.").ephemeral(true)).await?;
        return Ok(());
    }

    if question.is_none() {
        ctx.send(|m| m.content("You need to provide a question.").ephemeral(true)).await?;
        return Ok(());
    }

    ctx.defer().await?;

    let question = question.unwrap();
    let config_json = fs::read_to_string("config.json").await?;
    let config : Config = serde_json::from_str(&config_json).expect("config.json is malformed");
    let client = ChatGPT::new(config.openai_api_key)?;

    let mut conversation = new_conversation(client, ctx);

    let response = conversation
        .send_message(question)
        .await?;

    ctx.say(MessageBuilder::new().push_safe(&response.message().content).build()).await?;

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en-US", "Ask a GPT question using saved context."),
    discard_spare_arguments
)]
pub async fn chat(ctx: CowContext<'_>, #[rest] question: Option<String>) -> Result<(), Error> {
    let db = cowdb!(ctx);
    let id = ctx.author().id;
    if !db.has_gpt_enabled(id).await? {
        ctx.send(|m| m.content("You are not allowed to use this command.").ephemeral(true)).await?;
        return Ok(());
    }

    if question.is_none() {
        ctx.send(|m| m.content("You need to provide a question.").ephemeral(true)).await?;
        return Ok(());
    }

    ctx.defer().await?;

    let question = question.unwrap();
    let config_json = fs::read_to_string("config.json").await?;
    let config : Config = serde_json::from_str(&config_json).expect("config.json is malformed");
    let client = ChatGPT::new(config.openai_api_key)?;

    fs::create_dir_all(CONVERSATION_PATH).await?;
    let path = format!("{}/{}.json", CONVERSATION_PATH, id);
    let mut conversation = if fs::try_exists(&path).await? {
        match client.restore_conversation_json(&path).await {
            Ok(c) => c,
            Err(ex) => {
                error!("Failed to restore conversation: {}", ex);
                new_conversation(client, ctx)
            }
        }
    } else {
        new_conversation(client, ctx)
    };

    let response = conversation
        .send_message(question)
        .await?;

    ctx.say(MessageBuilder::new().push_safe(&response.message().content).build()).await?;

    conversation.save_history_json(&path).await?;

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en-US", "Reset the current conversation."),
    discard_spare_arguments
)]
pub async fn resetchat(ctx: CowContext<'_>) -> Result<(), Error> {
    let db = cowdb!(ctx);
    let id = ctx.author().id;
    if !db.has_gpt_enabled(id).await? {
        ctx.send(|m| m.content("You are not allowed to use this command.").ephemeral(true)).await?;
        return Ok(());
    }

    ctx.defer().await?;
    fs::remove_file(format!("{}/{}.json", CONVERSATION_PATH, id)).await?;
    ctx.send(|m| m.content("Successfully reset conversation.").ephemeral(true)).await?;

    Ok(())
}