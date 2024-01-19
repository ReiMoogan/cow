use poise::CreateReply;
use tracing::error;
use crate::{CowContext, Error};
use super::parse_input;
use proto_mc::ping::ping as mc_ping;
use serenity::all::CreateMessage;
use serenity::all::CreateEmbed;

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en-US", "Get basic information about a Minecraft server."),
    discard_spare_arguments
)]
pub async fn ping(
    ctx: CowContext<'_>,
    #[description = "The hostname of the server."] host: String,
    #[description = "The port of the server."] #[min = 1] #[max = 65535] port: Option<u16>)
-> Result<(), Error> {

    let input = parse_input(&host, port);
    match input {
        Ok(connection) => {
            ctx.defer().await?;

            match mc_ping(&connection).await {
                Ok(response) => {
                    let mut embed = CreateEmbed::new().title(format!("Minecraft Query of {connection}"));

                    if let Some(description) = response.description {
                        embed.field("MOTD", description.text, true);
                    }

                    embed.field("Version", response.version.name, true);

                    if let Some(ping) = response.ping {
                        embed.field("Ping", format!("{ping}"), true);
                    }

                    if let Some(player_list) = response.players.sample {
                        let player_list = player_list
                            .iter()
                            .map(|o| format!("- {}", o.name))
                            .reduce(|a, b| format!("{a}\n{b}"))
                            .unwrap_or_else(|| "\u{200b}".to_string());

                        embed.field(
                            format!("Players ({}/{})", response.players.online, response.players.max),
                            player_list, false);
                    } else {
                        embed.field(
                            format!("Players ({}/{})", response.players.online, response.players.max),
                            "\u{200b}", true);
                    }

                    ctx.send(CreateReply::default().embed(embed)).await?;
                },
                Err(e) => {
                    error!("Failed to ping server: {}", e);

                    ctx.send(CreateReply::default().content("Failed to ping server - is the host online?").ephemeral(true)).await?;
                }
            }
        }
        Err(ex) => {
            ctx.send(CreateReply::default().content(ex.to_string()).ephemeral(true)).await?;
        }
    }

    Ok(())
}
