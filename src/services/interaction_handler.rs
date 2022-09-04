use std::sync::Arc;
use std::error;
use std::fmt::Display;
use serenity::{
    client::Context,
    model::application::interaction::{
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
use serenity::model::application::interaction::application_command::CommandDataOptionValue;
use serenity::model::id::MessageId;

// Use these methods to automatically forward messages, depending on how they were invoked.
#[async_trait]
pub trait AutoResponse {
    async fn send_message<'a, F>(self, http: impl AsRef<Http> + Sync + Send, f: F) -> Result<Message, Box<dyn error::Error + Send + Sync>>
        where for<'b> F: FnOnce(&'b mut CreateMessage<'a>) -> &'b mut CreateMessage<'a> + Send;
    async fn say(self, http: impl AsRef<Http> + Sync + Send, content: impl Display + Send) -> Result<Message, Box<dyn error::Error + Send + Sync>>;
}

/*
#[async_trait]
impl AutoResponse for Message {
    async fn send_message<'a, F>(self, http: impl AsRef<Http> + Sync + Send, f: F) -> Result<Message, Box<dyn error::Error + Send + Sync>> where for<'b> F: FnOnce(&'b mut CreateMessage<'a>) -> &'b mut CreateMessage<'a> + Send{
        match self.channel_id.send_message(http, f).await
        {
            Ok(msg) => {
                return Ok(msg);
            }
            Err(ex) => {
                return Err(Box::new(ex));
            }
        }
    }

    async fn say(self, http: impl AsRef<Http> + Sync + Send, content: impl Display + Send) -> Result<Message, Box<dyn error::Error + Send + Sync>> {
        self.send_message(&http, |m| m.content(content)).await
    }
}*/

pub async fn interaction(ctx: &Context, interaction: &Interaction, framework: &Arc<Box<dyn Framework + Sync + Send>>) {
    if let Interaction::ApplicationCommand(command) = interaction {
        let app_id = command.application_id.as_u64();
        let cmd_name = command.data.name.as_str();
        // Ping the bot and append the command name, so we can trick it into thinking of a text command.
        let mut content = format!("<@!{}> {}", app_id, cmd_name);
        let arguments = command.data.options.iter()
            .filter(|o| o.value.is_some() && o.resolved.is_some())
            .map(|o| {
                match o.resolved.clone().unwrap() {
                    CommandDataOptionValue::String(s) => {s},
                    CommandDataOptionValue::Integer(i) => {i.to_string()},
                    CommandDataOptionValue::Boolean(b) => {b.to_string()},
                    CommandDataOptionValue::User(u, _) => {format!("<@{}>", u.id.0)},
                    CommandDataOptionValue::Channel(c) => {format!("<#{}>", c.id.0)},
                    CommandDataOptionValue::Role(r) => {format!("<@&{}", r.id.0)},
                    CommandDataOptionValue::Number(n) => {n.to_string()},
                    _ => String::new()
                }
            })
            .reduce(|a, b| format!("{} {}", a, b));

        if let Some(args) = arguments {
            content += "";
            content += &*args;
        }

        let mut dummy_message = CustomMessage::new();

        // We use an ID of 69420 to trick the framework into thinking it's a real message.
        dummy_message.channel_id(command.channel_id)
            .id(MessageId::from(69420))
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