use serenity::{
    client::Context,
    model::{id::{RoleId}, guild::Member}
};
use tracing::error;
use serenity::model::channel::Message;
use crate::{Database, db, Error};
use crate::models::minecraft_db_models::*;
use proto_mc::rcon::RCONClient;
use crate::models::minecraft_db_models::Message as MCMessage;

pub async fn non_command(ctx: &Context, msg: &Message) -> Result<(), Error>{
    ranking_check(ctx, msg).await;
    minecraft_check(ctx, msg).await;

    Ok(())
}

async fn minecraft_check(ctx: &Context, msg: &Message) {
    let author = &msg.author;

    if author.bot {
        return;
    }

    let db = db!(ctx);

    if let Ok(Some(feed)) = db.get_minecraft_channel(msg.channel_id).await {
        let mut client = RCONClient::<&String>::new(&feed.host, &feed.password);
        if (client.connect().await).is_err() { return; }
        if (client.login().await).is_err() { return; }

        let username = format!("{}#{:04}", msg.author.name, msg.author.discriminator);
        let nickname = msg.author_nick(&ctx.http).await;
        let display = if let Some(nick) = nickname {
            format!("{nick} ({username})")
        } else {
            username
        };

        let mut message = msg.content.clone();
        message.truncate(128);
        message = message.replace("\n", " ");

        let tellraw = vec![
            TellRaw::Text("<".to_string()),
            TellRaw::Message(MCMessage {
                text: display,
                color: "blue".to_string(),
                click_event: ClickEvent {
                    action: "copy_to_clipboard".to_string(),
                    value: msg.link()
                },
                hover_event: HoverEvent {
                    action: "show_text".to_string(),
                    contents: vec![
                        "Click to copy message link".to_string()
                    ]
                }
            }),
            TellRaw::Text("> ".to_string()),
            TellRaw::Text(message)
        ];

        let json = serde_json::to_string(&tellraw).unwrap();

        let command = format!("tellraw @a {json}");


        if (client.send(&command).await).is_err() { return; }
        if (client.disconnect().await).is_err() { return; }
    }
}

async fn ranking_check(ctx: &Context, msg: &Message) {
    let author = &msg.author;

    if author.bot {
        return;
    }

    let db = db!(ctx);

    if let Some(guild) = msg.guild(ctx) {
        match db.get_disablements(guild.id, msg.channel_id).await {
            Err(ex) => {
                error!("Failed checking if the current channel or guild was disabled: {}", ex);
            },
            Ok(result) => {
                if result.channel || result.guild {
                    return;
                }
            }
        }

        match db.provide_exp(guild.id, author.id).await {
            Err(ex) => {
                error!("Failed providing exp to user: {}", ex)
            },
            Ok(data) => {
                if data.level < 0 {
                    return;
                }

                let mut content = format!("<@{}> leveled up from {} to {}.", author.id.as_u64(), data.level - 1, data.level);
                if let Some(new_rank_id) = data.new_rank {
                    content += &format!("\nYou are now a <@&{new_rank_id}>.");

                    let mut error = false;

                    match msg.member(&ctx.http).await {
                        Ok(mut member) => {
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
                        Err(ex) => {
                            error!("Failed to get member from message: {}", ex);
                        }
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