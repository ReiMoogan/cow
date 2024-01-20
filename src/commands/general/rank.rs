use poise::CreateReply;
use serenity::{
    model::{
        id::{
            UserId,
            GuildId
        },
        user::User
    },
    utils::MessageBuilder
};
use serenity::all::CreateEmbed;
use serenity::builder::CreateEmbedFooter;
use crate::{Database, db, cowdb, Error, CowContext};
use tracing::{error};

// This prevents us from executing commands when the server has it disabled.
async fn guild_disabled(ctx: &CowContext<'_>, guild: &GuildId) -> bool {
    let db = cowdb!(ctx);

    match db.get_disablements(*guild, ctx.channel_id()).await {
        Ok(disablements) => {
            if disablements.guild {
                return true;
            }
        }
        Err(ex) => {
            error!("Error getting disablements: {}", ex);
            // If we can't read data for disablements, we probably can't add roles.
            // But, we might as well continue.
        }
    }

    false
}

async fn rank_embed(ctx: &CowContext<'_>, server_id: &GuildId, user: &User) {
    let db = cowdb!(ctx);

    let experience = db.get_xp(*server_id, user.id).await.unwrap();
    let xp = experience.xp;
    let level = experience.level;
    let next_level_xp = db.calculate_level(level).await.unwrap();

    let current_role = db.get_highest_role(*server_id, level).await.unwrap();
    let mut current_role_str: String = String::from("No role");
    if let Some(current_role_id) = current_role {
        current_role_str = format!("Current role: <@&{current_role_id}>");
    }

    let mut pfp_url = user.default_avatar_url();
    if let Some(pfp_custom) = user.avatar_url() {
        pfp_url = pfp_custom;
    }

    let mut rank_str = String::from("(Unranked)");
    if let Some(rank) = db.rank_within_members(*server_id, user.id).await.unwrap() {
        rank_str = format!("#{rank}");
    }

    let title = if let Some(discriminator) = user.discriminator {
        format!("{}#{:04}'s Ranking", user.name, discriminator)
    } else {
        format!("{}'s Ranking", user.name)
    };

    let embed = CreateEmbed::new()
        .title(title)
        .description(current_role_str)
        .field("Level", format!("{}", level), true)
        .field("XP", format!("{xp}/{next_level_xp}"), true)
        .field("Rank", rank_str, true)
        .thumbnail(pfp_url);

    if let Err(ex) = ctx.send(CreateReply::default().embed(embed)).await {
        error!("Failed to send embed: {}", ex);
    }
}

#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    description_localized("en-US", "Get your current rank."),
    aliases("course", "class", "classes")
)]
pub async fn rank(
    ctx: CowContext<'_>,
    #[description = "A user to check their rank"] user: Option<UserId>)
-> Result<(), Error> {
    if let Some(server_id) = ctx.guild_id() {
        if guild_disabled(&ctx, &server_id).await {
            return Ok(());
        }

        if let Some(other_id) = user {
            if let Ok(other_user) = other_id.to_user(&ctx).await {
                rank_embed(&ctx, &server_id, &other_user).await;
            } else {
                ctx.say("Could not find user...").await?;
            }
        } else {
            rank_embed(&ctx, &server_id, ctx.author()).await;
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
    description_localized("en-US", "Get the current rankings in the server.")
)]
pub async fn levels(
    ctx: CowContext<'_>,
    #[description = "The page of rankings to fetch"] #[min = 1] page: Option<i32>)
-> Result<(), Error> {
    let db = cowdb!(ctx);
    if let Some(server_id) = ctx.guild_id() {
        if guild_disabled(&ctx, &server_id).await {
            return Ok(());
        }

        let level_page = page.unwrap_or(1).max(1);
        match db.top_members(server_id, level_page - 1).await {
            Ok(pagination) => {
                let content = pagination.members.into_iter()
                    .enumerate()
                    .map(|o| {
                        let (index, member) = o;
                        format!("`#{}` <@{}> - Level {}, {} xp", (index as i32) + 10 * (level_page - 1) + 1, member.id, member.exp.level, member.exp.xp)
                    })
                    .reduce(|a, b| {format!("{a}\n{b}")})
                    .unwrap_or_else(|| "There is nothing on this page.".to_string());

                let embed = CreateEmbed::new()
                    .title("Top Users")
                    .description(content)
                    .footer(CreateEmbedFooter::new(format!("Page {}/{}", level_page, pagination.last_page)));

                ctx.send(CreateReply::default().embed(embed)).await?;
            },
            Err(ex) => {
                ctx.say("Failed to get rankings.".to_string()).await?;
                error!("Failed to get rankings: {}", ex);
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
    required_permissions = "ADMINISTRATOR",
    description_localized("en-US", "Disable/enable experience from being collected in the current channel."),
    aliases("enablexp"),
    discard_spare_arguments
)]
pub async fn disablexp(ctx: CowContext<'_>) -> Result<(), Error> {
    let db = cowdb!(ctx);
    if let Some(server_id) = ctx.guild_id() {
        let mut content: String;
        let channel = ctx.channel_id();
        match db.toggle_channel_xp(server_id, channel).await {
            Ok(toggle) => {
                if toggle {
                    content = "Disabled".to_string();
                } else {
                    content = "Enabled".to_string();
                }
                content += &format!(" collecting experience in <#{}>.", channel.get());
            },
            Err(ex) => {
                content = "Failed to toggle channel xp status.".to_string();
                error!("Failed to toggle channel xp status: {}", ex);
            }
        }

        ctx.say(content).await?;
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
    description_localized("en-US", "Disable/enable experience from being collected in the server."),
    aliases("enableserverxp"),
    discard_spare_arguments
)]
pub async fn disableserverxp(ctx: CowContext<'_>) -> Result<(), Error> {
    let db = cowdb!(ctx);
    if let Some(guild) = ctx.guild() {
        let mut content = MessageBuilder::new();

        match db.toggle_server_ranking(guild.id).await {
            Ok(toggle) => {
                if toggle {
                    content.push("Disabled");
                } else {
                    content.push("Enabled");
                }

                content.push(" collecting experience and ranking commands in ");
                content.push_safe(&guild.name);
                content.push(".");
            },
            Err(ex) => {
                content.push("Failed to toggle server xp status.");
                error!("Failed to toggle server xp status: {}", ex);
            }
        }

        ctx.say(content.build()).await?;
    } else {
        ctx.say("This command can only be run in a server.").await?;
    }

    Ok(())
}