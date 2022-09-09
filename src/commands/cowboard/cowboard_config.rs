use log::error;
use crate::{CowContext, cowdb, Error};
use serenity::model::channel::ReactionType;
use serenity::model::id::ChannelId;
use serenity::utils::MessageBuilder;
use crate::{Database, db};

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en-US", "Get the current settings for the cowboard."),
    guild_only
)]
pub async fn info(ctx: CowContext<'_>) -> Result<(), Error> {
    let db = cowdb!(ctx);

    if let Some(guild_id) = ctx.guild_id() {
        if let Ok(config) = db.get_cowboard_config(guild_id).await {
            ctx.send(|m| {
                m.embeds.clear();
                m.embed(|e|
                    e
                        .title("Cowboard Settings")
                        .description("If the emote doesn't display properly below, you probably want to use a different one!")
                        .field("Emote", &config.emote, true)
                        .field("Raw Emote", MessageBuilder::new().push_mono(&config.emote).build(), true)
                        .field("Channel", config.channel.map(|o| format!("<#{}>", o)).unwrap_or_else(|| "No Cowboard Channel".to_string()), true)
                        .field("Add Threshold", MessageBuilder::new().push_mono(config.add_threshold).build(), true)
                        .field("Remove Threshold", MessageBuilder::new().push_mono(config.remove_threshold).build(), true)
                        .field("Webhook", if config.webhook_id.is_some() && config.webhook_token.is_some() { "Enabled" } else { "Disabled" }, true)
                )
            }).await?;
        } else {
            ctx.say("Failed to fetch Cowboard settings for this server...").await?;
        }
    } else {
        ctx.say("This command can only be run in a server.").await?;
    }

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en-US", "Set the emote reaction to trigger a cowboard message."),
    required_permissions = "ADMINISTRATOR",
    guild_only
)]
pub async fn emote(
    ctx: CowContext<'_>,
    #[description = "An emote on the server or a default Discord emoji."] emoji: ReactionType)
-> Result<(), Error> {
    let db = cowdb!(ctx);

    if let Some(guild_id) = ctx.guild_id() {
        match db.get_cowboard_config(guild_id).await {
            Ok(mut config) => {
                config.emote = emoji.to_string();
                if let Err(ex) = db.update_cowboard(&config).await {
                    ctx.say("We couldn't update the cowboard, sorry... Try again later?").await?;
                    error!("Failed to update emote for cowboard: {}", ex);
                } else {
                    ctx.say("Successfully updated emote!").await?;
                }
            }
            Err(ex) => {
                ctx.say("We couldn't get the cowboard settings... try again later?").await?;
                error!("Failed to get cowboard: {}", ex);
            }
        }
    } else {
        ctx.say("This command can only be run in a server.").await?;
    }

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    description_localized("en-US", "Set the minimum amount of reactions to post a message to the cowboard."),
    required_permissions = "ADMINISTRATOR"
)]
pub async fn addthreshold(
    ctx: CowContext<'_>,
    #[description = "A positive number, greater than the removal bound."] add_threshold: i32)
-> Result<(), Error> {
    let db = cowdb!(ctx);

    if add_threshold <= 0 {
        ctx.say("The given number must be positive.").await?;
        return Ok(())
    }

    if let Some(guild_id) = ctx.guild_id() {
        match db.get_cowboard_config(guild_id).await {
            Ok(mut config) => {
                if add_threshold < config.remove_threshold {
                    ctx.say(format!("The minimum number of reactions required to add must be greater than or equal to the removal limit (currently set to {}).", config.remove_threshold)).await?;
                    return Ok(())
                }

                config.add_threshold = add_threshold;

                if let Err(ex) = db.update_cowboard(&config).await {
                    ctx.say("We couldn't update the cowboard, sorry... Try again later?").await?;
                    error!("Failed to update cowboard: {}", ex);
                } else {
                    ctx.say("Successfully updated minimum add threshold!").await?;
                }
            }
            Err(ex) => {
                ctx.say("We couldn't get the cowboard settings... try again later?").await?;
                error!("Failed to get cowboard: {}", ex);
            }
        }
    } else {
        ctx.say("This command can only be run in a server.").await?;
    }

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    description_localized("en-US", "Set the maximum amount of reactions before removing a message from the cowboard."),
    required_permissions = "ADMINISTRATOR"
)]
pub async fn removethreshold(
    ctx: CowContext<'_>,
    #[description = "A positive number, less than the addition bound."] remove_threshold: i32)
-> Result<(), Error> {
    let db = cowdb!(ctx);

    if remove_threshold < 0 {
        ctx.say("The given number must be positive or zero.").await?;
        return Ok(())
    }

    if let Some(guild_id) = ctx.guild_id() {
        match db.get_cowboard_config(guild_id).await {
            Ok(mut config) => {
                if remove_threshold > config.add_threshold {
                    ctx.say(format!("The maximum number of reactions required to remove must be less than or equal to the add limit (currently set to {}).", config.add_threshold)).await?;
                    return Ok(())
                }

                config.remove_threshold = remove_threshold;

                if let Err(ex) = db.update_cowboard(&config).await {
                    ctx.say("We couldn't update the cowboard, sorry... Try again later?").await?;
                    error!("Failed to update cowboard: {}", ex);
                } else {
                    ctx.say("Successfully updated maximum removal threshold!").await?;
                }
            }
            Err(ex) => {
                ctx.say("We couldn't get the cowboard settings... try again later?").await?;
                error!("Failed to get cowboard: {}", ex);
            }
        }
    } else {
        ctx.say("This command can only be run in a server.").await?;
    }

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    description_localized("en-US", "Sets the Cowboard channel to pin messages."),
    required_permissions = "ADMINISTRATOR"
)]
pub async fn channel(
    ctx: CowContext<'_>,
    #[description = "A channel to set the Cowboard channel to."] channel: Option<ChannelId>)
-> Result<(), Error> {
    let db = cowdb!(ctx);

    if let Some(guild_id) = ctx.guild_id() {
        let cowboard_channel = channel.unwrap_or_else(|| ctx.channel_id());

        if !ctx.guild().map(|g| g.channels.contains_key(&cowboard_channel)).unwrap_or(false) {
            ctx.say("Could not find channel in this server!").await?;
            return Ok(())
        }

        match db.get_cowboard_config(guild_id).await {
            Ok(mut config) => {
                config.channel = Some(cowboard_channel.0);
                config.webhook_id = None;
                config.webhook_token = None;

                if let Err(ex) = db.update_cowboard(&config).await {
                    ctx.say("We couldn't update the cowboard, sorry... Try again later?").await?;
                    error!("Failed to update cowboard: {}", ex);
                } else {
                    ctx.say("Successfully updated channel! You may want to check webhooks; try using `.cowboard webhook` to enable it.").await?;
                }
            }
            Err(ex) => {
                ctx.say("We couldn't get the cowboard settings... try again later?").await?;
                error!("Failed to get cowboard: {}", ex);
            }
        }
    } else {
        ctx.say("This command can only be run in a server.").await?;
    }

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    description_localized("en-US", "Toggle webhook usage for the cowboard, versus the bot sending the messages."),
    required_permissions = "ADMINISTRATOR"
)]
pub async fn webhook(ctx: CowContext<'_>) -> Result<(), Error> {
    let db = cowdb!(ctx);

    if let Some(guild) = ctx.guild() {
        match db.get_cowboard_config(guild.id).await {
            Ok(mut config) => {
                if config.channel == None {
                    ctx.say("Cowboard channel is not set up!").await?;
                    return Ok(());
                }

                let channel = ChannelId::from(config.channel.unwrap());
                match guild.channels(&ctx.discord()).await {
                    Ok(guild_channels) => {
                        if let Some(guild_channel) = guild_channels.get(&channel)
                        {
                            if config.webhook_id == None {
                                match guild_channel.create_webhook(&ctx.discord().http, "MooganCowboard").await {
                                    Ok(webhook) => {
                                        config.webhook_id = Some(webhook.id.0);
                                        config.webhook_token = Some(webhook.token.unwrap())
                                    }
                                    Err(ex) => {
                                        ctx.say(format!("Failed to add webhook; maybe I do not have permissions for the channel <#{}>?", guild_channel)).await?;
                                        error!("Failed to create webhook: {}", ex);
                                        return Ok(())
                                    }
                                };
                            } else {
                                config.webhook_id = None;
                                config.webhook_token = None;
                            }

                            if let Err(ex) = db.update_cowboard(&config).await {
                                ctx.say("We couldn't update the cowboard, sorry... Try again later?").await?;
                                error!("Failed to update cowboard: {}", ex);
                            } else if config.webhook_id == None {
                                ctx.say(format!("Disabled webhooks for <#{}>.", guild_channel)).await?;
                            } else {
                                ctx.say(format!("Enabled webhooks for <#{}>.", guild_channel)).await?;
                            }
                        }
                        else
                        {
                            ctx.say(format!("We don't have access to <#{}>... maybe it's hidden for us?", channel)).await?;
                        }
                    }
                    Err(ex) => {
                        error!("Failed to get guild channels: {}", ex);
                        ctx.say("We couldn't find the channels in this server, maybe we don't have permissions?").await?;
                    }
                }
            }
            Err(ex) => {
                error!("Failed to get cowboard: {}", ex);
                ctx.say("We couldn't get the cowboard settings... try again later?").await?;
            }
        }
    } else {
        ctx.say("This command can only be run in a server.").await?;
    }

    Ok(())
}