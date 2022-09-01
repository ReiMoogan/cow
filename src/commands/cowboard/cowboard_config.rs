use log::error;
use serenity::{
    framework::standard::{
        macros::command, Args, CommandResult,
    },
    model::channel::Message, client::Context
};
use serenity::model::channel::ReactionType;
use serenity::model::id::ChannelId;
use serenity::utils::MessageBuilder;
use crate::{Database, db};

#[command]
#[description = "Get the current settings for the cowboard."]
#[only_in(guilds)]
pub async fn info(ctx: &Context, msg: &Message) -> CommandResult {
    let db = db!(ctx);

    if let Some(guild_id) = msg.guild_id {
        if let Ok(config) = db.get_cowboard_config(guild_id).await {
            msg.channel_id.send_message(&ctx.http, |m| m.embed(|e|
                e
                    .title("Cowboard Settings")
                    .description("If the emote doesn't display properly below, you probably want to use a different one!")
                    .field("Emote", &config.emote, true)
                    .field("Raw Emote", MessageBuilder::new().push_mono(&config.emote).build(), true)
                    .field("Channel", config.channel.map(|o| format!("<#{}>", o)).unwrap_or_else(|| "No Cowboard Channel".to_string()), true)
                    .field("Add Threshold", MessageBuilder::new().push_mono(config.add_threshold).build(), true)
                    .field("Remove Threshold", MessageBuilder::new().push_mono(config.remove_threshold).build(), true)
                    .field("Webhook", if config.webhook_id.is_some() && config.webhook_token.is_some() { "Enabled" } else { "Disabled" }, true)
            )).await?;
        } else {
            msg.channel_id.say(&ctx.http, "Failed to fetch Cowboard settings for this server...").await?;
        }
    } else {
        msg.reply(&ctx.http, "This command can only be run in a server.").await?;
    }

    Ok(())
}

#[command]
#[description = "Set the emote reaction to trigger a cowboard message."]
#[usage = "An emote, preferably one on the server or a default Discord emoji."]
#[only_in(guilds)]
#[required_permissions("ADMINISTRATOR")]
pub async fn emote(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let db = db!(ctx);

    if args.is_empty() {
        msg.channel_id.say(&ctx.http, "You need to pass an emote to this command, like :cow:.").await?;
        return Ok(());
    }

    if let Ok(emoji) = args.single::<ReactionType>() {
        if let Some(guild_id) = msg.guild_id {
            match db.get_cowboard_config(guild_id).await {
                Ok(mut config) => {
                    config.emote = emoji.to_string();
                    if let Err(ex) = db.update_cowboard(&config).await {
                        msg.channel_id.say(&ctx.http, "We couldn't update the cowboard, sorry... Try again later?").await?;
                        error!("Failed to update emote for cowboard: {}", ex);
                    } else {
                        msg.channel_id.say(&ctx.http, "Successfully updated emote!").await?;
                    }
                }
                Err(ex) => {
                    msg.channel_id.say(&ctx.http, "We couldn't get the cowboard settings... try again later?").await?;
                    error!("Failed to get cowboard: {}", ex);
                }
            }
        } else {
            msg.reply(&ctx.http, "This command can only be run in a server.").await?;
        }
    } else {
        msg.channel_id.say(&ctx.http, "Failed to process an emote from the given message...").await?;
        return Ok(());
    }

    Ok(())
}

#[command]
#[description = "Set the minimum amount of reactions to post a message to the cowboard."]
#[usage = "A positive number, greater than the removal bound."]
#[only_in(guilds)]
#[required_permissions("ADMINISTRATOR")]
pub async fn addthreshold(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let db = db!(ctx);

    if args.is_empty() {
        msg.channel_id.say(&ctx.http, "You need to pass in a positive number for the minimum amount of reactions.").await?;
        return Ok(());
    }

    if let Ok(add_threshold) = args.single::<i32>() {
        if add_threshold <= 0 {
            msg.channel_id.say(&ctx.http, "The given number must be positive.").await?;
            return Ok(())
        }

        if let Some(guild_id) = msg.guild_id {
            match db.get_cowboard_config(guild_id).await {
                Ok(mut config) => {
                    if add_threshold < config.remove_threshold {
                        msg.channel_id.say(&ctx.http, format!("The minimum number of reactions required to add must be greater than or equal to the removal limit (currently set to {}).", config.remove_threshold)).await?;
                        return Ok(())
                    }

                    config.add_threshold = add_threshold;

                    if let Err(ex) = db.update_cowboard(&config).await {
                        msg.channel_id.say(&ctx.http, "We couldn't update the cowboard, sorry... Try again later?").await?;
                        error!("Failed to update cowboard: {}", ex);
                    } else {
                        msg.channel_id.say(&ctx.http, "Successfully updated minimum add threshold!").await?;
                    }
                }
                Err(ex) => {
                    msg.channel_id.say(&ctx.http, "We couldn't get the cowboard settings... try again later?").await?;
                    error!("Failed to get cowboard: {}", ex);
                }
            }
        } else {
            msg.reply(&ctx.http, "This command can only be run in a server.").await?;
        }
    } else {
        msg.channel_id.say(&ctx.http, "The given value is not a valid number.").await?;
        return Ok(());
    }

    Ok(())
}

#[command]
#[description = "Set the maximum amount of reactions before removing a message from the cowboard."]
#[usage = "A positive number, less than the addition bound."]
#[only_in(guilds)]
#[required_permissions("ADMINISTRATOR")]
pub async fn removethreshold(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let db = db!(ctx);

    if args.is_empty() {
        msg.channel_id.say(&ctx.http, "You need to pass in a positive number (or zero) for the removal reaction count.").await?;
        return Ok(());
    }

    if let Ok(remove_threshold) = args.single::<i32>() {
        if remove_threshold < 0 {
            msg.channel_id.say(&ctx.http, "The given number must be positive or zero.").await?;
            return Ok(())
        }

        if let Some(guild_id) = msg.guild_id {
            match db.get_cowboard_config(guild_id).await {
                Ok(mut config) => {
                    if remove_threshold > config.add_threshold {
                        msg.channel_id.say(&ctx.http, format!("The maximum number of reactions required to remove must be less than or equal to the add limit (currently set to {}).", config.add_threshold)).await?;
                        return Ok(())
                    }

                    config.remove_threshold = remove_threshold;

                    if let Err(ex) = db.update_cowboard(&config).await {
                        msg.channel_id.say(&ctx.http, "We couldn't update the cowboard, sorry... Try again later?").await?;
                        error!("Failed to update cowboard: {}", ex);
                    } else {
                        msg.channel_id.say(&ctx.http, "Successfully updated maximum removal threshold!").await?;
                    }
                }
                Err(ex) => {
                    msg.channel_id.say(&ctx.http, "We couldn't get the cowboard settings... try again later?").await?;
                    error!("Failed to get cowboard: {}", ex);
                }
            }
        } else {
            msg.reply(&ctx.http, "This command can only be run in a server.").await?;
        }
    } else {
        msg.channel_id.say(&ctx.http, "The given value is not a valid number.").await?;
        return Ok(());
    }

    Ok(())
}

#[command]
#[description = "Sets the Cowboard channel to pin messages."]
#[usage = "Either uses the current channel or a provided channel."]
#[only_in(guilds)]
#[required_permissions("ADMINISTRATOR")]
pub async fn channel(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let db = db!(ctx);

    let mut channel = msg.channel_id;

    if !args.is_empty() {
        let custom_channel = args.single::<ChannelId>();
        if custom_channel.is_err() {
            msg.channel_id.say(&ctx.http, "Could not get a channel from your input!").await?;
            return Ok(())
        }
        channel = custom_channel.unwrap();
    }

    if let Some(guild_id) = msg.guild_id {
        if !msg.guild(ctx).map(|g| g.channels.contains_key(&channel)).unwrap_or(false) {
            msg.channel_id.say(&ctx.http, "Could not find channel in this server!").await?;
            return Ok(())
        }

        match db.get_cowboard_config(guild_id).await {
            Ok(mut config) => {
                config.channel = Some(channel.0);
                config.webhook_id = None;
                config.webhook_token = None;

                if let Err(ex) = db.update_cowboard(&config).await {
                    msg.channel_id.say(&ctx.http, "We couldn't update the cowboard, sorry... Try again later?").await?;
                    error!("Failed to update cowboard: {}", ex);
                } else {
                    msg.channel_id.say(&ctx.http, "Successfully updated channel! You may want to check webhooks; try using `.cowboard webhook` to enable it.").await?;
                }
            }
            Err(ex) => {
                msg.channel_id.say(&ctx.http, "We couldn't get the cowboard settings... try again later?").await?;
                error!("Failed to get cowboard: {}", ex);
            }
        }
    } else {
        msg.reply(&ctx.http, "This command can only be run in a server.").await?;
    }

    Ok(())
}

#[command]
#[description = "Toggle webhook usage for the cowboard, versus the bot sending the messages."]
#[only_in(guilds)]
#[required_permissions("ADMINISTRATOR")]
pub async fn webhook(ctx: &Context, msg: &Message) -> CommandResult {
    let db = db!(ctx);

    if let Some(guild) = msg.guild(ctx) {
        match db.get_cowboard_config(guild.id).await {
            Ok(mut config) => {
                if config.channel == None {
                    msg.channel_id.say(&ctx.http, "Cowboard channel is not set up!").await?;
                    return Ok(());
                }

                let channel = ChannelId::from(config.channel.unwrap());
                match guild.channels(&ctx.http).await {
                    Ok(guild_channels) => {
                        if let Some(guild_channel) = guild_channels.get(&channel)
                        {
                            if config.webhook_id == None {
                                match guild_channel.create_webhook(&ctx.http, "MooganCowboard").await {
                                    Ok(webhook) => {
                                        config.webhook_id = Some(webhook.id.0);
                                        config.webhook_token = Some(webhook.token.unwrap())
                                    }
                                    Err(ex) => {
                                        msg.channel_id.say(&ctx.http, format!("Failed to add webhook; maybe I do not have permissions for the channel <#{}>?", guild_channel)).await?;
                                        error!("Failed to create webhook: {}", ex);
                                        return Ok(())
                                    }
                                };
                            } else {
                                config.webhook_id = None;
                                config.webhook_token = None;
                            }

                            if let Err(ex) = db.update_cowboard(&config).await {
                                msg.channel_id.say(&ctx.http, "We couldn't update the cowboard, sorry... Try again later?").await?;
                                error!("Failed to update cowboard: {}", ex);
                            } else if config.webhook_id == None {
                                msg.channel_id.say(&ctx.http, format!("Disabled webhooks for <#{}>.", guild_channel)).await?;
                            } else {
                                msg.channel_id.say(&ctx.http, format!("Enabled webhooks for <#{}>.", guild_channel)).await?;
                            }
                        }
                        else
                        {
                            msg.channel_id.say(&ctx.http, format!("We don't have access to <#{}>... maybe it's hidden for us?", channel)).await?;
                        }
                    }
                    Err(ex) => {
                        error!("Failed to get guild channels: {}", ex);
                        msg.channel_id.say(&ctx.http, format!("We couldn't find the channels in this server, maybe we don't have permissions?")).await?;
                    }
                }
            }
            Err(ex) => {
                error!("Failed to get cowboard: {}", ex);
                msg.channel_id.say(&ctx.http, "We couldn't get the cowboard settings... try again later?").await?;
            }
        }
    } else {
        msg.reply(&ctx.http, "This command can only be run in a server.").await?;
    }

    Ok(())
}