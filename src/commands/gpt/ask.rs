use tokio::fs;
use crate::{CowContext, db, cowdb, Database, Error, models::config::Config};
use chatgpt::prelude::ChatGPT;
use chatgpt::types::CompletionResponse;

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

    let response: CompletionResponse = client
        .send_message(question)
        .await?;

    ctx.say(&response.message().content).await?;

    Ok(())
}