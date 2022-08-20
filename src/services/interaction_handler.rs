use std::sync::Arc;
use serenity::{
    client::Context,
    model::interactions::{
        Interaction,
        InteractionResponseType
    },
    framework::Framework,
    utils::CustomMessage
};
use log::error;
use async_trait::async_trait;
use chrono::{Utc};
use serenity::builder::CreateMessage;
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::interactions::application_command::ApplicationCommandInteractionDataOptionValue;

// Use these methods to automatically forward messages, depending on how they were invoked.
#[async_trait]
pub trait AutoResponse {
    async fn send_message<'a, F>(self, http: impl AsRef<Http>, f: F) -> Result<Message, Box<dyn std::error::Error + Send + Sync>>
        where for<'b> F: FnOnce(&'b mut CreateMessage<'a>) -> &'b mut CreateMessage<'a>;
    async fn say(self, http: impl AsRef<Http>, content: impl std::fmt::Display) -> Result<Message, Box<dyn std::error::Error + Send + Sync>>;
}

/*
#[async_trait]
impl AutoResponse for Message {
    async fn send_message<'a, F>(self, http: impl AsRef<Http>, f: F) -> Result<Message, serenity::Error> where for<'b> F: FnOnce(&'b mut CreateMessage<'a>) -> &'b mut CreateMessage<'a> {
        self.channel_id.send_message(http, f).await
    }

    async fn say(self, http: impl AsRef<Http>, content: impl Display) -> Result<Message, serenity::Error> {
        self.send_message(&http, |m| m.content(content)).await
    }
}*/

pub async fn interaction(ctx: &Context, interaction: &Interaction, framework: &Arc<Box<dyn Framework + Sync + std::marker::Send>>) {
    if let Interaction::ApplicationCommand(command) = interaction {
        let app_id = command.application_id.as_u64();
        let cmd_name = command.data.name.as_str();
        // Ping the bot and append the command name, so we can trick it into thinking of a text command.
        let mut content = format!("<@!{}> {}", app_id, cmd_name);
        let arguments = command.data.options.iter()
            .filter(|o| o.value.is_some() && o.resolved.is_some())
            .map(|o| {
                match o.resolved.clone().unwrap() {
                    ApplicationCommandInteractionDataOptionValue::String(s) => {s},
                    ApplicationCommandInteractionDataOptionValue::Integer(i) => {i.to_string()},
                    ApplicationCommandInteractionDataOptionValue::Boolean(b) => {b.to_string()},
                    ApplicationCommandInteractionDataOptionValue::User(u, _) => {format!("<@{}>", u.id.0)},
                    ApplicationCommandInteractionDataOptionValue::Channel(c) => {format!("<#{}>", c.id.0)},
                    ApplicationCommandInteractionDataOptionValue::Role(r) => {format!("<@&{}", r.id.0)},
                    ApplicationCommandInteractionDataOptionValue::Number(n) => {n.to_string()},
                    _ => String::new()
                }
            })
            .reduce(|a, b| format!("{} {}", a, b));

        if let Some(args) = arguments {
            content += "";
            content += &*args;
        }

        let mut dummy_message = CustomMessage::new();

        dummy_message.channel_id(command.channel_id)
            .content(content)
            .author(command.user.clone())
            .timestamp(Utc::now());

        if let Some(guild_id) = command.guild_id {
            dummy_message.guild_id(guild_id);
        }

        (*framework).dispatch(ctx.clone(), dummy_message.build()).await;

        if let Err(ex) = command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::UpdateMessage)
            })
            .await
        {
            error!("Failed to respond to slash command: {}", ex);
        }
    }
}