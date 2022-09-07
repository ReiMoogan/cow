use serenity::{
    client::Context,
    model::{channel::Message, id::{RoleId}, guild::Member}
};
use log::error;
use crate::{Database, db};

pub async fn non_command(ctx: &Context) {
    if msg.author.bot {
        return;
    }

    let db = db!(ctx);

    if let Some(server_id) = ctx.guild_id() {
        match db.channel_disabled(server_id, msg.channel_id).await {
            Err(ex) => {
                error!("Failed checking if the current channel was disabled: {}", ex);
            },
            Ok(result) => {
                if result {
                    return;
                }
            }
        }

        match db.provide_exp(server_id, msg.author.id).await {
            Err(ex) => {
                error!("Failed providing exp to user: {}", ex)
            },
            Ok(data) => {
                if data.level < 0 {
                    return;
                }

                let mut content = format!("<@{}> leveled up from {} to {}.", msg.author.id.as_u64(), data.level - 1, data.level);
                if let Some(new_rank_id) = data.new_rank {
                    content += &*format!("\nYou are now a <@&{}>.", new_rank_id);

                    let mut error = false;
                    let guild = msg.guild(&ctx).unwrap();
                    let mut member = guild.member(&ctx.http, msg.author.id).await.unwrap();

                    if let Some(old_rank_id) = data.old_rank {
                        let old_rank = RoleId::from(old_rank_id);
                        if member.roles.contains(&old_rank) {
                            // We know we're in a guild, so an error is probably an API issue.
                            if let Err(ex) = member.remove_role(&ctx.http, old_rank).await {
                                error = true;
                                content += "\n(We failed to update your roles; maybe we don't have permission?)";
                                error!("Failed to remove role from user: {}", ex);
                            }
                        }
                    }

                    if let Err(ex) = member.add_role(&ctx.http, RoleId::from(new_rank_id)).await {
                        if !error {
                            content += "\n(We failed to update your roles; maybe we don't have permission?)";
                        }
                        error!("Failed to add role to user: {}", ex);
                    }
                }

                if let Err(ex2) =
                    msg.channel_id.send_message(&ctx.http, |m| m.embed(|e| e
                        .title("Level Up!")
                        .description(content)
                    )).await {
                        error!("Error sending level-up message: {}", ex2)
                };
            }
        }
    }
}

pub async fn on_join(ctx: &Context, new_member: &Member) {
    if new_member.user.bot {
        return;
    }

    let db = db!(ctx);
    let mut member = new_member.clone();
    let guild_id = new_member.guild_id;

    let experience = db.get_xp(guild_id, member.user.id).await.unwrap();
    let current_role = db.get_highest_role(guild_id, experience.level).await.unwrap();
    if let Some(current_role_id) = current_role {
        if let Err(ex) = member.add_role(&ctx.http, current_role_id).await {
            error!("Failed to add role for server {}: {}", guild_id, ex);
            if let Err(ex2) = member.user.direct_message(&ctx.http, |m|
                m.content("I tried to re-add your roles, but the server didn't let me. Sorry~")).await {
                error!("Failed to send error message to user {}: {}", member.user.id, ex2);
            }
        }
    }
}