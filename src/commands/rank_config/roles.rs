use crate::{CowContext, cowdb, Error};
use serenity::{
    model::{
        id::{
            RoleId
        }
    },
};
use crate::{Database, db};
use tracing::{error};
use serenity::model::guild::Role;

// Parameters: rankconfig add [min_level] [rank]

#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    description_localized("en-US", "Add a rank to the configuration."),
    required_permissions = "ADMINISTRATOR"
)]
pub async fn add(
    ctx: CowContext<'_>,
    #[description = "The minimum level to obtain this rank"] min_level: i32,
    #[description = "The role that is associated with this rank"] role: Role)
-> Result<(), Error> {
    let db = cowdb!(ctx);

    if let Some(guild) = ctx.guild() {
        match db.add_role(guild.id, &*role.name, role.id, min_level).await {
            Ok(success) => {
                if success {
                    ctx.say(format!("Successfully added <@&{}> with minimum level {}.", role.id.as_u64(), min_level)).await?;
                } else {
                    ctx.say(format!("There is a duplicate role with minimum level {}.", min_level)).await?;
                }
            }
            Err(ex) => {
                error!("Failed to add role for server: {}", ex);
                ctx.say("Failed to add role to the server.").await?;
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
    description_localized("en-US", "Remove a rank to the configuration."),
    required_permissions = "ADMINISTRATOR"
)]
pub async fn remove(
    ctx: CowContext<'_>,
    #[description = "The role associated with the rank to remove."] role_id: RoleId)
-> Result<(), Error> {
    let db = cowdb!(ctx);
    // So much nesting...
    if let Some(guild) = ctx.guild() {
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
    } else {
        ctx.say("This command can only be run in a server.").await?;
    }

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    description_localized("en-US", "List the current ranks on this server."),
    required_permissions = "ADMINISTRATOR"
)]
pub async fn list(ctx: CowContext<'_>) -> Result<(), Error> {
    let db = cowdb!(ctx);
    if let Some(guild_id) = ctx.guild_id() {
        match db.get_roles(guild_id).await {
            Ok(items) => {
                if let Err(ex) = ctx.send(|m| {
                    m.embeds.clear();
                    m.embed(|e| {
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
        ctx.say("This command can only be run in a server.").await?;
    }

    Ok(())
}