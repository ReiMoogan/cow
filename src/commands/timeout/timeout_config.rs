use log::error;
use crate::{CowContext, Database, db, cowdb, Error};
use crate::util::{ to_ms, from_ms };

#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    required_permissions = "ADMINISTRATOR",
    description_localized("en", "Sets server-wide cooldown for messaging xp gain."),
)]
pub async fn set(
    ctx: CowContext<'_>,
    #[description = "A duration with suffixes d, h, m, and s. Ex. \"1m30s\" for 1 minute and 30 seconds."] timeout: String)
-> Result<(), Error> {
    let db = cowdb!(ctx);
    // nesting part 2
    if let Some(server_id) = ctx.guild_id() {
        if let Some(timeout) = to_ms(timeout) {
            match db.set_timeout(server_id, timeout).await {
                Ok(_) => { ctx.say(format!("Set timeout to {}.", from_ms(timeout as u64))).await?; }
                Err(err) => {
                    ctx.say("Could not set timeout").await?;
                    error!("Could not set timeout: {}", err);
                }
            }
        } else {
            ctx.say("The timeout must be in the form #d#h#m#s.").await?;
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
    required_permissions = "ADMINISTRATOR",
    description_localized("en", "Gets the server-wide cooldown for messaging xp gain."),
)]
pub async fn get(ctx: CowContext<'_>) -> Result<(), Error> {
    let db = cowdb!(ctx);
    if let Some(server_id) = ctx.guild_id() {
        match db.get_timeout(server_id).await {
            Ok(timeout) => { ctx.say(format!("The timeout is {}.", from_ms(timeout as u64))).await?; }
            Err(err) => {
                ctx.say("Could not get timeout.").await?;
                error!("Could not get timeout: {}", err);
            }
        }
    } else {
        ctx.say("This command can only be run in a server.").await?;
    }

    Ok(())
}