use crate::CowContext;
use serenity::{
    client::Context,
    model::{
        channel::Message,
        id::{
            UserId,
            GuildId
        },
        user::User
    },
    framework::standard::{
        CommandResult,
        macros::{
            command
        },
        Args
    },
    utils::MessageBuilder
};
use crate::{Database, db};
use log::{error};

async fn rank_embed(ctx: &CowContext<'_>, server_id: &GuildId, user: &User) {
    let db = cowdb!(ctx);

    let experience = db.get_xp(*server_id, user.id).await.unwrap();
    let xp = experience.xp;
    let level = experience.level;
    let next_level_xp = db.calculate_level(level).await.unwrap();

    let current_role = db.get_highest_role(*server_id, level).await.unwrap();
    let mut current_role_str: String = String::from("No role");
    if let Some(current_role_id) = current_role {
        current_role_str = format!("Current role: <@&{}>", current_role_id);
    }

    let mut pfp_url = user.default_avatar_url();
    if let Some(pfp_custom) = user.avatar_url() {
        pfp_url = pfp_custom;
    }

    let mut rank_str = String::from("(Unranked)");
    if let Some(rank) = db.rank_within_members(*server_id, user.id).await.unwrap() {
        rank_str = format!("#{}", rank);
    }

    if let Err(ex) = msg.channel_id.send_message(&ctx.http, |m| {m.embed(|e| {
        e
            .title(
                MessageBuilder::new()
                    .push_safe(user.name.as_str())
                    .push("#")
                    .push(user.discriminator)
                    .push("'s Ranking")
                    .build()
            )
            .description(current_role_str)
            .field("Level", level, true)
            .field("XP", format!("{}/{}", xp, next_level_xp), true)
            .field("Rank", rank_str, true)
            .thumbnail(pfp_url)
    })}).await {
        error!("Failed to send embed: {}", ex);
    }
}

#[poise::command(prefix_command, slash_command)]
#[description = "Get your current rank."]
#[only_in(guilds)]
pub async fn rank(ctx: &CowContext<'_>, mut args: Args) -> CommandResult {
    let other = args.single::<UserId>();
    if let Some(server_id) = ctx.guild_id() {
        if let Ok(other_id) = other {
            if let Ok(other_user) = other_id.to_user(&ctx.http).await {
                rank_embed(ctx, msg, &server_id, &other_user).await;
            } else {
                ctx.say("Could not find user...").await?;
            }
        } else {
            rank_embed(ctx, msg, &server_id, &msg.author).await;
        }
    } else {
        msg.reply(&ctx.http, "This command can only be run in a server.").await?;
    }

    Ok(())
}

#[poise::command(prefix_command, slash_command)]
#[description = "Disable/enable experience from being collected in the current channel."]
#[only_in(guilds)]
#[required_permissions("ADMINISTRATOR")]
#[aliases("enablexp")]
pub async fn disablexp(ctx: &CowContext<'_>) -> CommandResult {
    let db = cowdb!(ctx);
    if let Some(server_id) = ctx.guild_id() {
        let mut content: String;
        match db.toggle_channel_xp(server_id, msg.channel_id).await {
            Ok(toggle) => {
                if toggle {
                    content = "Disabled".to_string();
                } else {
                    content = "Enabled".to_string();
                }
                content += &*format!(" collecting experience in <#{}>.", msg.channel_id.as_u64());
            },
            Err(ex) => {
                content = "Failed to toggle channel xp status.".to_string();
                error!("Failed to toggle channel xp status: {}", ex);
            }
        }

        msg.channel_id.send_message(&ctx.http, |m| {m.content(content)}).await?;
    } else {
        msg.reply(&ctx.http, "This command can only be run in a server.").await?;
    }

    Ok(())
}

#[poise::command(prefix_command, slash_command)]
#[description = "Get the current rankings in the server."]
#[only_in(guilds)]
pub async fn levels(ctx: &CowContext<'_>, mut args: Args) -> CommandResult {
    let db = cowdb!(ctx);
    if let Some(server_id) = ctx.guild_id() {
        let page = args.single::<i32>().unwrap_or(1).max(1);
        match db.top_members(server_id, page - 1).await {
            Ok(pagination) => {
                let content = pagination.members.into_iter()
                    .enumerate()
                    .into_iter()
                    .map(|o| {
                        let (index, member) = o;
                        format!("`#{}` <@{}> - Level {}, {} xp", (index as i32) + 10 * (page - 1) + 1, member.id, member.exp.level, member.exp.xp)
                    })
                    .reduce(|a, b| {format!("{}\n{}", a, b)})
                    .unwrap_or_else(|| "There is nothing on this page.".to_string());
                msg.channel_id.send_message(&ctx.http, |m| {
                    m.embed(|e|
                        e
                            .title("Top Users")
                            .description(content)
                            .footer(|e| e.text(format!("Page {}/{}", page, pagination.last_page)))
                    )}).await?;
            },
            Err(ex) => {
                ctx.say("Failed to get rankings.".to_string()).await?;
                error!("Failed to get rankings: {}", ex);
            }
        }
    } else {
        msg.reply(&ctx.http, "This command can only be run in a server.").await?;
    }

    Ok(())
}