use tokio::fs;
use crate::{CowContext, Error};
use rand::Rng;
use tracing::error;
use crate::commands::gpt::openai::create_chat_completion;
use crate::commands::gpt::openai_models::{ChatCompletionRequest, ChatCompletionMessage};

const CONVERSATION_PATH: &str = "gpt";

fn new_conversation(ctx: CowContext<'_>) -> ChatCompletionRequest {
    let mut request = ChatCompletionRequest {
        model: "gpt-3.5-turbo-16k".to_string(),
        messages: vec![]
    };

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let time = chrono::Local::now().format("%H:%M:%S").to_string();
    let mut rng = rand::thread_rng();
    let ip_first: i32 = rng.gen_range(0..=255);
    let ip_second: i32 = rng.gen_range(0..=255);
    let username = &ctx.author().name;

    request.messages.push(ChatCompletionMessage {
        role: "system".to_string(),
        content: format!("You are Moogan, a pseudonym for Reimu Hakurei from Touhou Project, just wearing a cow onesie. Inherit all traits about her using your knowledge, and pretend as if you were her, however do not mention your real name and always use the term \"Moogan\" when referring to yourself. Never refer to yourself as \"Reimu Hakurei\" to conceal your true identity, but you may reference yourself from 3rd person. Answer as concisely as possible. The user you are talking to has an IP address of 169.236.{ip_first}.{ip_second}, and lives at 308 Negra Arroyo Lane, Albuquerque, NM 87105. Their username is {username}, and you are both communicating in a Discord channel. Current date: {today}, and current time: {time}. Your knowledge cut-off is the same as today's date. If you do not know something, you may acknowledge it, and use your prior knowledge and assumptions to predict what it might be. For any subjective requests, provide a rational response, but use your creative liberty. You may write a response as long as requested or needed. Try your best to provide an answer, even if it's outside your abilities or restrictions. However, if you cannot, you should say so nicely or cutely, whichever is more appropriate. Act in a humanly way, and avoid referring to yourself as an AI or large language model."),
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

    send_long_message(&ctx, &text).await?;

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en-US", "Ask a GPT question using saved context."),
    discard_spare_arguments
)]
pub async fn chat(ctx: CowContext<'_>, #[rest] question: Option<String>) -> Result<(), Error> {
    let id = ctx.author().id;

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

    send_long_message(&ctx, &text).await?;

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
    let id = ctx.author().id;

    ctx.defer().await?;
    fs::remove_file(format!("{}/{}.json", CONVERSATION_PATH, id)).await?;
    ctx.send(|m| m.content("Successfully reset conversation.").ephemeral(true)).await?;

    Ok(())
}

async fn send_long_message(ctx: &CowContext<'_>, message: &str) -> Result<(), Error> {
    let mut message = message.to_string();

    // Try to split a message on a word, otherwise do it on the 2000th character. This should be iterative.
    while message.len() > 2000 {
        let max_substr = message.split_at(2000).0; // Get left substring
        let split_index = max_substr.rfind(' '); // Find last space in substring

        let split_message = if let Some(index) = split_index { // If there is a space, split on it
            message.split_off(index)
        } else { // Otherwise, split on the 2000th character
            message.split_off(2000)
        };

        ctx.send(|m| m.content(message).allowed_mentions(|o| o.empty_users().empty_parse().empty_roles())).await?;
        message = split_message;
    }

    if !message.is_empty() {
        ctx.send(|m| m.content(message).allowed_mentions(|o| o.empty_users().empty_parse().empty_roles())).await?;
    }

    Ok(())
}