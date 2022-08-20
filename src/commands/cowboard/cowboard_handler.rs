use std::path::Path;
use tokio::fs::File;
use log::error;
use serenity::client::Context;
use serenity::http::AttachmentType;
use serenity::model::channel::{Embed, Message, Reaction, ReactionType};
use serenity::model::id::{ChannelId, GuildId, MessageId, UserId};
use tokio::io::AsyncWriteExt;
use crate::{Database, db};
use crate::commands::cowboard::cowboard_db_models::{Cowboard};

async fn count_reactions(ctx: &Context, message: &Message, config: &Cowboard) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>{
    let config_emote = ReactionType::try_from(config.emote.as_str())?;
    let matched_reaction = message.reactions.iter().find(|o|o.reaction_type.eq(&config_emote));
    if let Some(reaction) = matched_reaction {
        let count = reaction.count;
        let people = message.reaction_users(&ctx.http, config_emote, None, UserId::from(message.author.id.0 - 2)).await?;
        if people.iter().any(|o| o.id == message.author.id) {
            return Ok(count - 1);
        }
        return Ok(count);
    }

    Ok(0)
}

pub async fn add_reaction(ctx: &Context, added_reaction: &Reaction) {
    if added_reaction.guild_id.is_none() {
        return;
    }
    let guild_id = added_reaction.guild_id.unwrap();
    let db = db!(ctx);
    match db.get_cowboard_config(guild_id).await {
        Ok(mut config) => {
            if config.channel.is_none() {
                // No cowboard, why even check?
                return;
            }

            match added_reaction.message(&ctx.http).await {
                Ok(message) => {
                    match count_reactions(ctx, &message, &config).await {
                        Ok(count) => {
                            // Pray that the database's constraints work.
                            if count >= config.add_threshold as u64 {
                                let post_message = db.get_cowboard_message(message.id, message.channel_id, guild_id).await;
                                if let Ok(Some(post)) = post_message {
                                    match ctx.http.get_message(post.post_channel_id, post.post_id).await {
                                        Ok(mut post) => {
                                            update_moo(ctx, &message, &mut post, &mut config).await;
                                        }
                                        Err(ex) => {
                                            error!("Failed to get old cowboard message: {}", ex);
                                            // Create a new copy
                                            add_moo(ctx, guild_id, added_reaction, &message, &mut config).await;
                                        }
                                    }
                                } else if let Err(ex) = post_message {
                                    error!("Failed to get message from database: {}", ex);
                                } else {
                                    // Moo that thing!
                                    add_moo(ctx, guild_id, added_reaction, &message, &mut config).await;
                                }
                            }
                        }
                        Err(ex) => {
                            error!("Failed to count reactions: {}", ex);
                        }
                    }
                }
                Err(ex) => {
                    error!("Failed to get reacted message: {}", ex);
                }
            }
        }
        Err(ex) => {
            error!("Failed to get cowboard config: {}", ex);
        }
    }
}

async fn add_moo(ctx: &Context, guild_id: GuildId, reaction: &Reaction, message: &Message, config: &mut Cowboard) {
    let db = db!(ctx);

    let message_result = if config.webhook_id.is_some() && config.webhook_token.is_some() {
        send_webhook_message(ctx, message, config).await
    } else {
        send_bot_message(ctx, message, config).await
    };

    if let Err(ex) = message_result {
        error!("Failed to send cowboard message: {}", ex);
        return;
    }

    let post_message = message_result.unwrap();

    if let Err(ex) = db.moo_message(message.id, reaction.channel_id, post_message.id, post_message.channel_id, guild_id).await {
        error!("Failed to moo a message in the database: {}", ex);
    }
}

async fn update_moo(ctx: &Context, message: &Message, post_message: &mut Message, config: &mut Cowboard) {
    if config.webhook_id.is_some() && config.webhook_token.is_some() {
        update_webhook_message(ctx, message, post_message, config).await
    } else {
        update_bot_message(ctx, message, post_message, config).await
    };
}

async fn send_bot_message(ctx: &Context, message: &Message, config: &Cowboard) -> Result<Message, Box<dyn std::error::Error + Send + Sync>> {
    let channel = ChannelId::from(config.channel.unwrap());
    let output_username = format_username(ctx, message).await;
    let safe_content = message.content_safe(ctx).await;

    let attachments = download_image_attachments(message).await;

    let reacts = count_reactions(ctx, message, config).await?;
    let link = message.link_ensured(&ctx.http).await;

    let message_output = channel.send_message(&ctx.http, |m|
        {
            let execution = m
                .content(format!("{} {} | <#{}>\n{}", reacts, &config.emote, message.channel_id, link))
                .embed(|e| {
                    let temp = e
                        .author(|a|
                            a.name(&output_username).icon_url(message.author.face()))
                        .description(&safe_content)
                        .timestamp(&message.timestamp)
                        .footer(|f| f.text(format!("Message ID: {} / User ID: {}", message.id, message.author.id)));

                    if !attachments.is_empty() {
                        let (name, _) = &attachments[0];
                        temp.attachment(name);
                    }

                    temp
                });

            for (_, path) in &attachments {
                execution.add_file(AttachmentType::Path(Path::new(path)));
            }

            execution
        }
    ).await;

    delete_image_attachments(message).await;
    match message_output {
        Ok(message) => {
            Ok(message)
        }
        Err(ex) => {
            Err(Box::new(ex))
        }
    }
}

async fn update_bot_message(ctx: &Context, message: &Message, post_message: &mut Message, config: &mut Cowboard) {
    match count_reactions(ctx, message, config).await {
        Ok(reacts) => {
            let link = message.link_ensured(&ctx.http).await;
            if let Err(ex) = post_message.edit(&ctx.http,
                                                  |m| m.content(format!("{} {} | <#{}>\n{}", reacts, &config.emote, message.channel_id, link))).await {
                error!("Failed to edit post message??? {}", ex);
            }
        }
        Err(ex) => {
            error!("Failed to count reactions: {}", ex);
        }
    }
}

async fn send_webhook_message(ctx: &Context, message: &Message, config: &mut Cowboard) -> Result<Message, Box<dyn std::error::Error + Send + Sync>> {
    let token = config.webhook_token.clone().unwrap();
    if let Ok(webhook) = ctx.http.get_webhook_with_token(config.webhook_id.unwrap(), &*token).await {
        let output_username = format_username(ctx, message).await;
        let safe_content = message.content_safe(ctx).await;

        let attachments = download_image_attachments(message).await;

        let embeds = vec![
            Embed::fake(|e|
                {
                    let temp = e
                        .author(|a|
                            a.name(&output_username).icon_url(message.author.face()))
                        .description(&safe_content)
                        .timestamp(&message.timestamp)
                        .footer(|f| f.text(format!("Message ID: {} / User ID: {}", message.id, message.author.id)));

                    if !attachments.is_empty() {
                        let (name, _) = &attachments[0];
                        temp.attachment(name);
                    }

                    temp
                }
            )
        ];

        let reacts = count_reactions(ctx, message, config).await?;
        let link = message.link_ensured(&ctx.http).await;
        if let Ok(Some(webhook_message)) = webhook.execute(&ctx.http, true, |m|
            {
                let execution = m
                    .content(format!("{} {} | <#{}>\n{}", reacts, &config.emote, message.channel_id, link))
                    .embeds(embeds)
                    .avatar_url(message.author.face())
                    .username(output_username);

                for (_, path) in &attachments {
                    execution.add_file(AttachmentType::Path(Path::new(path)));
                }

                execution
            }
        ).await {
            delete_image_attachments(message).await;
            return Ok(webhook_message);
        }
    }

    delete_image_attachments(message).await;
    disable_webhook(ctx, config).await;
    send_bot_message(ctx, message, config).await
}

async fn download_image_attachments(message: &Message) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();

    let directory = format!("cowboard/{}", message.id);

    if let Err(ex) = tokio::fs::create_dir_all(&directory).await {
        error!("Failed to create temporary directory: {}", ex);
        return out;
    }

    let mut size_limit: u64 = 8 * 1024 * 1024;

    for item in message.attachments.iter() {
        if item.dimensions().is_some() && size_limit >= item.size {
            // Is an image that we can upload!
            let content = match item.download().await {
                Ok(content) => content,
                Err(ex) => {
                    error!("Error downloading file: {}", ex);
                    continue;
                }
            };

            let file_path = format!("{}/{}", &directory, &item.filename);
            let mut file = match File::create(&file_path).await {
                Ok(file) => file,
                Err(ex) => {
                    error!("Error creating file: {}", ex);
                    continue;
                }
            };

            if let Err(ex) = file.write_all(&content).await {
                error!("Error saving image: {}", ex);
                continue;
            }

            size_limit -= item.size;
            out.push((item.filename.clone(), file_path));
        }
    }

    out
}

async fn delete_image_attachments(message: &Message) {
    let directory = format!("cowboard/{}", message.id);

    if let Err(ex) = tokio::fs::remove_dir_all(&directory).await {
        error!("Failed to remove directory: {}", ex);
    }
}

async fn format_username(ctx: &Context, message: &Message) -> String {
    let username = format!("{}#{}", message.author.name, message.author.discriminator);
    let nickname = message.author_nick(&ctx.http).await;

    if let Some(nick) = nickname {
        format!("{} ({})", nick, username)
    } else {
        username
    }
}

async fn update_webhook_message(ctx: &Context, message: &Message, post_message: &Message, config: &mut Cowboard) {
    let token = config.webhook_token.clone().unwrap();
    if let Ok(webhook) = ctx.http.get_webhook_with_token(config.webhook_id.unwrap(), &*token).await {
        match count_reactions(ctx, message, config).await {
            Ok(reacts) => {
                let link = message.link_ensured(&ctx.http).await;
                if let Err(ex) = webhook.edit_message(&ctx.http, post_message.id,
                                                      |m| m.content(format!("{} {} | <#{}>\n{}", reacts, &config.emote, message.channel_id, link))).await {
                    error!("Failed to edit post message??? {}", ex);
                }
            }
            Err(ex) => {
                error!("Failed to count reactions: {}", ex);
            }
        }
    } else {
        disable_webhook(ctx, config).await;
    }
}

async fn disable_webhook(ctx: &Context, config: &mut Cowboard) {
    let db = db!(ctx);

    config.webhook_id = None;
    config.webhook_token = None;
    if let Err(ex) = db.update_cowboard(config).await {
        error!("Failed to update cowboard settings: {}", ex);
    }
}

pub async fn remove_reaction(ctx: &Context, removed_reaction: &Reaction) {
    if removed_reaction.guild_id.is_none() {
        return;
    }

    let guild_id = removed_reaction.guild_id.unwrap();
    let db = db!(ctx);
    match db.get_cowboard_config(guild_id).await {
        Ok(mut config) => {
            match removed_reaction.message(&ctx.http).await {
                Ok(message) => {
                    match count_reactions(ctx, &message, &config).await {
                        Ok(count) => {
                            let post_message = db.get_cowboard_message(message.id, message.channel_id, guild_id).await;
                            // Pray that the database's constraints work.
                            if count < config.remove_threshold as u64 {
                                // Unmoo that thing!
                                remove_moo(ctx, guild_id, removed_reaction.channel_id, removed_reaction.message_id).await;
                            } else if let Ok(Some(post)) = post_message {
                                if let Ok(mut post) = ctx.http.get_message(post.post_channel_id, post.post_id).await {
                                    update_moo(ctx, &message, &mut post, &mut config).await;
                                }
                            }
                        }
                        Err(ex) => {
                            error!("Failed to count reactions: {}", ex);
                        }
                    }
                }
                Err(ex) => {
                    error!("Failed to get reacted message: {}", ex);
                }
            }
        }
        Err(ex) => {
            error!("Failed to get cowboard config: {}", ex);
        }
    }
}

pub async fn reaction_remove_all(ctx: &Context, channel_id: ChannelId, message: MessageId) {
    let guild_id = channel_id.message(&ctx.http, message).await.ok().and_then(|o| o.guild_id);
    if let Some(guild) = guild_id {
        remove_moo(ctx, guild, channel_id, message).await;
    }
}

async fn remove_moo(ctx: &Context, guild_id: GuildId, channel_id: ChannelId, message: MessageId) {
    let db = db!(ctx);

    match db.get_cowboard_message(message, channel_id, guild_id).await {
        Ok(message_info) => {
            if let Some(cowboard_message) = message_info {
                if let Err(ex) = ctx.http.delete_message(cowboard_message.post_channel_id, cowboard_message.post_id).await {
                    error!("Failed to delete message: {} {} {}", ex, cowboard_message.post_channel_id, cowboard_message.post_id);
                }
            }
        }
        Err(ex) => {
            error!("Failed to query cowboard message: {}", ex);
        }
    }

    if let Err(ex) = db.unmoo_message(message, channel_id, guild_id).await {
        error!("Failed to unmoo a message in the database: {}", ex);
    }
}