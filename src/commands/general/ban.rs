use std::fmt::Display;
use crate::{CowContext, Error};

#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    description_localized("en", "Ban all League of Legends players from the server."),
    required_permissions = "BAN_MEMBERS"
)]
pub async fn banleagueplayers(
    ctx: CowContext<'_>,
    #[description = "A custom ban message for all degenerates"] ban_message: Option<String>)
-> Result<(), Error> {
    if let Some(message) = ban_message {
        ban_game_players(&ctx, 356869127241072640, message).await
    } else {
        ban_game_players(&ctx, 356869127241072640, "Playing League? Cringe.").await
    }
}

#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    description_localized("en", "Ban all VALORANT players from the server."),
    required_permissions = "BAN_MEMBERS"
)]
pub async fn banvalorantplayers(
    ctx: CowContext<'_>,
    #[description = "A custom ban message for all degenerates"] ban_message: Option<String>)
-> Result<(), Error> {
    if let Some(message) = ban_message {
        ban_game_players(&ctx, 700136079562375258, message).await
    } else {
        ban_game_players(&ctx, 700136079562375258, "Playing VALORANT? Cringe.").await
    }
}

#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    description_localized("en", "Ban all Genshin Impact players from the server."),
    required_permissions = "BAN_MEMBERS"
)]
pub async fn bangenshinplayers(
    ctx: CowContext<'_>,
    #[description = "A custom ban message for all degenerates"] ban_message: Option<String>)
-> Result<(), Error> {
    if let Some(message) = ban_message {
        ban_game_players(&ctx, 762434991303950386, message).await
    } else {
        ban_game_players(&ctx, 762434991303950386, "Playing Genshin? Cringe.").await
    }
}

#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    description_localized("en", "Ban all Overwatch players from the server."),
    required_permissions = "BAN_MEMBERS"
)]
pub async fn banoverwatchplayers(
    ctx: CowContext<'_>,
    #[description = "A custom ban message for all degenerates"] ban_message: Option<String>)
-> Result<(), Error> {
    if let Some(message) = ban_message {
        ban_game_players(&ctx, 356875221078245376, message).await
    } else {
        ban_game_players(&ctx, 356875221078245376, "Dead Game.").await
    }
}

async fn ban_game_players(ctx: &CowContext<'_>, game_id: u64, message: impl AsRef<str> + Display) -> Result<(), Error> {
    if let Some(guild) = ctx.guild() {
        let serenity = ctx.discord();

        let mut degenerates: Vec<u64> = Vec::new();
        for (_, presence) in guild.presences.iter() {
            if presence.activities.iter()
                .filter_map(|o| o.application_id)
                .any(|o| o == game_id) {
                degenerates.push(u64::from(presence.user.id));
                if let Ok(dm_channel) = presence.user.id.create_dm_channel(&serenity.http).await {
                    dm_channel.say(&serenity.http, format!("You have been banned for playing haram games. Message: {}", message)).await?;
                }
                let _ = guild.ban_with_reason(&serenity.http, presence.user.id, 0, &message).await;
            }
        }

        let list = degenerates.iter().map(|o| format!("<@{}>", o)).reduce(|a, b| format!("{}, {}", a, b));
        if let Some(output) = list {
            ctx.say(format!("Successfully banned these degenerates: {}", output)).await?;
        } else {
            ctx.say("No haram activities detected.").await?;
        }
    }

    Ok(())
}
