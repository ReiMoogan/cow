use serenity::client::Context;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::channel::{Message};
use serenity::framework::standard::macros::{command};

#[command]
#[only_in(guilds)]
#[required_permissions("BAN_MEMBERS")]
async fn banleagueplayers(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    if args.is_empty() {
        return ban_game_players(ctx, msg, 356869127241072640, "Playing League? Cringe.").await;
    }

    ban_game_players(ctx, msg, 356869127241072640, args.message()).await
}

#[command]
#[only_in(guilds)]
#[required_permissions("BAN_MEMBERS")]
async fn banvalorantplayers(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    if args.is_empty() {
        return ban_game_players(ctx, msg, 700136079562375258, "Playing VALORANT? Cringe.").await;
    }
    ban_game_players(ctx, msg, 700136079562375258, args.message()).await
}

#[command]
#[only_in(guilds)]
#[required_permissions("BAN_MEMBERS")]
async fn bangenshinplayers(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    if args.is_empty() {
        return ban_game_players(ctx, msg, 762434991303950386, "Playing Genshin? Cringe.").await;
    }

    ban_game_players(ctx, msg, 762434991303950386, args.message()).await
}

#[command]
#[only_in(guilds)]
#[required_permissions("BAN_MEMBERS")]
async fn banoverwatchplayers(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    if args.is_empty() {
        return ban_game_players(ctx, msg, 356875221078245376, "Dead Game.").await;
    }

    ban_game_players(ctx, msg, 356875221078245376, args.message()).await
}

async fn ban_game_players(ctx: &Context, msg: &Message, game_id: u64, message: impl AsRef<str>) -> CommandResult {
    if let Some(guild) = msg.guild(&ctx).await {
        let mut degenerates: Vec<u64> = Vec::new();
        for (_, presence) in guild.presences.iter() {
            if presence.activities.iter()
                .filter_map(|o| o.application_id)
                .any(|o| o == game_id) {
                degenerates.push(u64::from(presence.user_id));
                if let Ok(dm_channel) = presence.user_id.create_dm_channel(&ctx.http).await {
                    dm_channel.say(&ctx.http, "You have been banned for playing haram games.").await?;
                }
                let _ = guild.ban_with_reason(&ctx.http, presence.user_id, 0, &message).await;
            }
        }

        let list = degenerates.iter().map(|o| format!("<@{}>", o)).reduce(|a, b| format!("{}, {}", a, b));
        if let Some(output) = list {
            msg.channel_id.say(&ctx.http, format!("Successfully banned these degenerates: {}", output)).await?;
        } else {
            msg.channel_id.say(&ctx.http, "No haram activities detected.").await?;
        }
    }

    Ok(())
}
