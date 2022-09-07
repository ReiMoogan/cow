use crate::CowContext;
use serenity::{
    client::Context,
    model::{
        channel::Message,
        id::{
            RoleId
        },
        guild::Guild
    },
    framework::standard::{
        CommandResult,
        macros::{
            command
        },
        Args
    },
    utils::{
        MessageBuilder
    }
};
use crate::{Database, db};
use log::{error};

// Parameters: rankconfig add [min_level] [rank]

async fn get_role(ctx: &CowContext<'_>, guild: &Guild, args: &Args) -> Option<(RoleId, String)> {
    let role_id: RoleId;
    let mut role_text: String;

    if let Ok(role) = args.parse::<RoleId>() {
        role_id = role;
        if let Some(role) = guild.roles.get(&role) {
            role_text = role.name.clone();
        } else {
            if let Err(ex) = ctx.say(format!("Could not find a role on this server matching <@&{}>!", role_id.as_u64())).await {
                error!("Failed to send message: {}", ex);
            }
            return None
        }
    } else {
        role_text = args.rest().to_string();
        if let Some(role) = guild.role_by_name(&*role_text) {
            role_id = role.id;
            role_text = role.name.clone(); // Just to make it exact.
        } else {
            let content = MessageBuilder::new().push("Could not find a role on this server matching \"").push_safe(role_text).push("\"!").build();
            if let Err(ex) = ctx.say(content).await {
                error!("Failed to send message: {}", ex);
            }
            return None
        }
    }

    Some((role_id, role_text))
}

#[poise::command(prefix_command, slash_command)]
#[description = "Add a rank to the configuration."]
#[only_in(guilds)]
#[usage = "<level> <role id or name>"]
#[required_permissions("ADMINISTRATOR")]
pub async fn add(ctx: &CowContext<'_>, mut args: Args) -> CommandResult {
    let db = cowdb!(ctx);
    // So much nesting...
    if let Some(guild) = msg.guild(&ctx.cache) {
        if let Ok(min_level) = args.single::<i32>() {
            if let Some((role_id, role_text)) = get_role(ctx, msg, &guild, &args).await {
                // Both min_level and role_id are initialized by this point
                match db.add_role(guild.id, &role_text, role_id, min_level).await {
                    Ok(success) => {
                        if success {
                            ctx.say(format!("Successfully added <@&{}> with minimum level {}.", role_id.as_u64(), min_level)).await?;
                        } else {
                            ctx.say(format!("There is a duplicate role with minimum level {}.", min_level)).await?;
                        }
                    }
                    Err(ex) => {
                        error!("Failed to add role for server: {}", ex);
                        ctx.say("Failed to add role to the server.").await?;
                    }
                }
            }
        } else {
            ctx.say("The first argument should be a positive integer, representing the minimum level for this rank.").await?;
        }
    } else {
        msg.reply(&ctx.http, "This command can only be run in a server.").await?;
    }

    Ok(())
}

#[poise::command(prefix_command, slash_command)]
#[description = "Remove a rank from the configuration."]
#[only_in(guilds)]
#[required_permissions("ADMINISTRATOR")]
pub async fn remove(ctx: &CowContext<'_>, args: Args) -> CommandResult {
    let db = cowdb!(ctx);
    // So much nesting...
    if let Some(guild) = msg.guild(&ctx.cache) {
        if let Some((role_id, _)) = get_role(ctx, msg, &guild, &args).await {
            match db.remove_role(guild.id, role_id).await {
                Ok(success) => {
                    if success {
                        ctx.say(format!("Successfully removed <@&{}>.", role_id.as_u64())).await?;
                    } else {
                        ctx.say("A rank didn't exist for this role.".to_string()).await?;
                    }
                }
                Err(ex) => {
                    error!("Failed to remove role for server: {}", ex);
                    ctx.say("Failed to remove role from the server.").await?;
                }
            }
        }
    } else {
        msg.reply(&ctx.http, "This command can only be run in a server.").await?;
    }

    Ok(())
}

#[poise::command(prefix_command, slash_command)]
#[description = "List the current ranks on this server."]
#[only_in(guilds)]
#[required_permissions("ADMINISTRATOR")]
pub async fn list(ctx: &CowContext<'_>) -> CommandResult {
    let db = cowdb!(ctx);
    if let Some(guild_id) = ctx.guild_id() {
        match db.get_roles(guild_id).await {
            Ok(items) => {
                if let Err(ex) = msg.channel_id.send_message(&ctx.http, |m| {m.embed(|e| {
                    e.title("Rank to Level Mapping")
                        .description(
                            items.into_iter()
                                .map(|i| {
                                    let mut content = format!("{}: <no role> at level {}", i.name, i.min_level);
                                    if let Some(role_id) = i.role_id {
                                        content = format!("{}: <@&{}> at level {}", i.name, role_id, i.min_level);
                                    }
                                    content
                                })
                                .reduce(|a, b| {format!("{}\n{}", a, b)})
                                .unwrap_or_else(|| "No roles are registered on this server.".to_string())
                        )})}).await {
                    error!("Failed to send message to server: {}", ex);
                }
            },
            Err(ex) => error!("Failed to get roles for server: {}", ex)
        }
    } else {
        msg.reply(&ctx.http, "This command can only be run in a server.").await?;
    }

    Ok(())
}