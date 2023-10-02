use tokio::fs;
use crate::{CowContext, db, cowdb, Database, Error};
use rand::Rng;
use tracing::error;
use crate::commands::gpt::openai::create_chat_completion;
use crate::commands::gpt::openai_models::{ChatCompletionRequest, ChatCompletionMessage};

const CONVERSATION_PATH: &str = "gpt";

fn new_conversation(ctx: CowContext<'_>) -> ChatCompletionRequest {
    let mut request = ChatCompletionRequest {
        model: "gpt-3.5-turbo".to_string(),
        messages: vec![]
    };

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let mut rng = rand::thread_rng();
    let ip_first: i32 = rng.gen_range(0..=255);
    let ip_second: i32 = rng.gen_range(0..=255);
    let username = &ctx.author().name;

    request.messages.push(ChatCompletionMessage {
        role: "system".to_string(),
        content: format!("You are MooganGPT, a large language model trained by OpenAI. Answer as concisely as possible. The user you are talking to has an IP address of 169.236.{ip_first}.{ip_second}, and lives at 308 Negra Arroyo Lane, Albuquerque, NM 87105. Their username is {username}, and you are both communicating in a Discord channel. Knowledge cutoff: 2021-09 Current date: {today}"),
        name: None
    });

    request
}

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en-US", "Ask a GPT question without any context."),
    discard_spare_arguments
)]
pub async fn ask(ctx: CowContext<'_>, #[rest] question: Option<String>) -> Result<(), Error> {
    // let db = cowdb!(ctx);
    //
    // if !db.has_gpt_enabled(ctx.author().id).await? {
    //     ctx.send(|m| m.content("You are not allowed to use this command.").ephemeral(true)).await?;
    //     return Ok(());
    // }

    if question.is_none() {
        ctx.send(|m| m.content("You need to provide a question.").ephemeral(true)).await?;
        return Ok(());
    }

    ctx.defer().await?;

    let question = question.unwrap();

    let mut conversation = new_conversation(ctx);
    conversation.messages.push(ChatCompletionMessage {
        role: "user".to_string(),
        content: question,
        name: Some(ctx.author().id.to_string())
    });

    let response = create_chat_completion(&conversation).await?;
    let text = response.choices.last().map(|o| o.message.content.clone()).unwrap_or_else(|| "Couldn't generate a response...".to_string());

    ctx.send(|m| m.content(text).allowed_mentions(|o| o.empty_users().empty_parse().empty_roles())).await?;

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en-US", "Ask a GPT question using saved context."),
    discard_spare_arguments
)]
pub async fn chat(ctx: CowContext<'_>, #[rest] question: Option<String>) -> Result<(), Error> {
    // let db = cowdb!(ctx);
    let id = ctx.author().id;
    // if !db.has_gpt_enabled(id).await? {
    //     ctx.send(|m| m.content("You are not allowed to use this command.").ephemeral(true)).await?;
    //     return Ok(());
    // }

    if question.is_none() {
        ctx.send(|m| m.content("You need to provide a question.").ephemeral(true)).await?;
        return Ok(());
    }

    ctx.defer().await?;

    let question = question.unwrap();

    fs::create_dir_all(CONVERSATION_PATH).await?;
    let path = format!("{}/{}.json", CONVERSATION_PATH, id);
    let mut conversation = if fs::try_exists(&path).await? {
        match fs::read_to_string(&path).await {
            Ok(data) => {
                match serde_json::from_str::<Vec<ChatCompletionMessage>>(&data) {
                    Ok(mut messages) => {
                        let mut temp_conversation = new_conversation(ctx);
                        temp_conversation.messages.clear();
                        temp_conversation.messages.append(&mut messages);
                        temp_conversation
                    }
                    Err(ex) => {
                        error!("Failed to deserialize conversation: {}", ex);
                        new_conversation(ctx)
                    }
                }
            }
            Err(ex) => {
                error!("Failed to read conversation: {}", ex);
                new_conversation(ctx)
            }
        }
    } else {
        new_conversation(ctx)
    };

    conversation.messages.push(ChatCompletionMessage {
        role: "user".to_string(),
        content: question,
        name: Some(ctx.author().id.to_string())
    });

    let response = create_chat_completion(&conversation).await?;
    let text = response.choices.last().map(|o| o.message.content.clone()).unwrap_or_else(|| "Couldn't generate a response...".to_string());

    ctx.send(|m| m.content(text).allowed_mentions(|o| o.empty_users().empty_parse().empty_roles())).await?;

    if let Some(message) = response.choices.last() {
        conversation.messages.push(ChatCompletionMessage {
            role: message.message.role.clone(),
            content: message.message.content.clone(),
            name: message.message.name.clone()
        });

        let output_json = serde_json::to_string(&conversation.messages)?;
        fs::write(&path, output_json).await?;
    }

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en-US", "Reset the current conversation."),
    discard_spare_arguments
)]
pub async fn resetchat(ctx: CowContext<'_>) -> Result<(), Error> {
    // let db = cowdb!(ctx);
    let id = ctx.author().id;
    // if !db.has_gpt_enabled(id).await? {
    //     ctx.send(|m| m.content("You are not allowed to use this command.").ephemeral(true)).await?;
    //     return Ok(());
    // }

    ctx.defer().await?;
    fs::remove_file(format!("{}/{}.json", CONVERSATION_PATH, id)).await?;
    ctx.send(|m| m.content("Successfully reset conversation.").ephemeral(true)).await?;

    Ok(())
}