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
use regex::Regex;
use serenity::all::{CreateEmbed, CreateMessage};

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
        if client.connect().await.is_err() { return; }
        if client.login().await.is_err() { return; }

        let username = if let Some(discriminator) = msg.author.discriminator {
            format!("{}#{:04}", msg.author.name, discriminator)
        } else {
            msg.author.name.clone()
        };

        let nickname = msg.author_nick(&ctx.http).await;
        let display = if let Some(nick) = nickname {
            format!("{nick} ({username})")
        } else {
            username
        };

        // Remove the IDs from emotes.
        let regex = Regex::new(r"(?m)<a?(:[^:]+:)\d+>").unwrap();
        let mut message = msg.content_safe(&ctx.cache);

        if !message.is_empty() {
            message = regex.replace_all(&message, "$1").to_string();
            message.truncate(256);
            message = message.replace('\n', " ");

            let mut tellraw = vec![
                TellRaw::Text("<".to_string()),
                TellRaw::Message(MCMessage {
                    text: display.to_string(),
                    color: "blue".to_string(),
                    italic: false,
                    underlined: false,
                    click_event: ClickEvent {
                        action: "open_url".to_string(),
                        value: msg.link()
                    },
                    hover_event: HoverEvent {
                        action: "show_text".to_string(),
                        contents: vec![
                            "Open message link".to_string()
                        ]
                    }
                }),
                TellRaw::Text("> ".to_string())
            ];

            for part in message.split(' ') {
                if part.starts_with("http") {
                    tellraw.push(TellRaw::Message(MCMessage {
                        text: part.to_string(),
                        color: "white".to_string(),
                        italic: false,
                        underlined: true,
                        click_event: ClickEvent {
                            action: "open_url".to_string(),
                            value: part.to_string()
                        },
                        hover_event: HoverEvent {
                            action: "show_text".to_string(),
                            contents: vec![
                                "Open link".to_string()
                            ]
                        }
                    }));
                } else {
                    tellraw.push(TellRaw::Text(part.to_string()));
                }

                tellraw.push(TellRaw::Text(" ".to_string()));
            }

            let json = serde_json::to_string(&tellraw).unwrap();

            let command = format!("tellraw @a {json}");

            if client.send(&command).await.is_err() { return; }
        }

        for attachment in &msg.attachments {
            let tellraw = vec![
                TellRaw::Text("<".to_string()),
                TellRaw::Message(MCMessage {
                    text: display.to_string(),
                    color: "blue".to_string(),
                    italic: false,
                    underlined: false,
                    click_event: ClickEvent {
                        action: "open_url".to_string(),
                        value: msg.link()
                    },
                    hover_event: HoverEvent {
                        action: "show_text".to_string(),
                        contents: vec![
                            "Open message link".to_string()
                        ]
                    }
                }),
                TellRaw::Text("> ".to_string()),
                TellRaw::Message(MCMessage {
                    text: format!("Attached file: {}", attachment.filename),
                    color: "white".to_string(),
                    italic: true,
                    underlined: false,
                    click_event: ClickEvent {
                        action: "open_url".to_string(),
                        value: attachment.url.to_string()
                    },
                    hover_event: HoverEvent {
                        action: "show_text".to_string(),
                        contents: vec![
                            "Open content".to_string()
                        ]
                    }
                }),
            ];

            let json = serde_json::to_string(&tellraw).unwrap();
            let command = format!("tellraw @a {json}");
            if client.send(&command).await.is_err() { return; }
        }

        if client.disconnect().await.is_err() { }
    }
}

async fn ranking_check(ctx: &Context, msg: &Message) {
    let author = &msg.author;

    if author.bot {
        return;
    }

    let db = db!(ctx);

    if let Some(guild) = msg.guild(&ctx.cache) {
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

                let mut content = format!("<@{}> leveled up from {} to {}.", author.id.get(), data.level - 1, data.level);
                if let Some(new_rank_id) = data.new_rank {
                    content += &format!("\nYou are now a <@&{new_rank_id}>.");

                    let mut error = false;

                    match msg.member(&ctx.http).await {
                        Ok(member) => {
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

                let new_message = CreateMessage::new().embed(CreateEmbed::new().title("Level Up!").description(content));

                if let Err(ex2) = msg.channel_id.send_message(&ctx.http, new_message).await {
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
    let member = new_member.clone();
    let guild_id = new_member.guild_id;

    let experience = db.get_xp(guild_id, member.user.id).await.unwrap();
    let current_role = db.get_highest_role(guild_id, experience.level).await.unwrap();
    if let Some(current_role_id) = current_role {
        if let Err(ex) = member.add_role(&ctx.http, current_role_id).await {
            error!("Failed to add role for server {}: {}", guild_id, ex);
            if let Err(ex2) = member.user.direct_message(&ctx.http, CreateMessage::new().content("I tried to add your role, but the server didn't let me. Sorry~")).await {
                error!("Failed to send error message to user {}: {}", member.user.id, ex2);
            }
        }
    }
}