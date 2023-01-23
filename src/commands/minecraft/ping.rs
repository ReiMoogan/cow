use tracing::error;
use crate::{CowContext, Error};
use proto_mc::ping::ping as mc_ping;

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

    let input_host: String;
    let input_port: String;

    // Check the input parameters
    let split = host.split(':').collect::<Vec<&str>>();

    if split.len() == 1 {
        input_host = split[0].to_string();
        input_port = port.unwrap_or(25565).to_string();
    } else if split.len() == 2 {
        input_host = split[0].to_string();
        input_port = split[1].to_string();
    } else {
        ctx.send(|msg| {
            msg.content("Invalid hostname, please try again.").ephemeral(true)
        }).await?;

        return Ok(());
    }

    // Bound check our port
    let input_port = input_port.parse::<u16>();

    if input_port.is_err() {
        ctx.send(|msg| {
            msg.content("Invalid port, please try again.").ephemeral(true)
        }).await?;
        return Ok(());
    }

    let input_port = input_port.unwrap();

    // At this point, our input is valid (input_host and input_port are valid)
    ctx.defer().await?;

    match mc_ping(format!("{input_host}:{input_port}")).await {
        Ok(response) => {
            ctx.send(|msg| {
                msg.embed(|embed| {
                    embed.title(format!("Minecraft Query of {input_host}"));

                    if let Some(description) = response.description {
                        embed.field("MOTD", description.text, true);
                    }

                    embed.field("Version", response.version.name, true);

                    if let Some(ping) = response.ping {
                        embed.field("Ping", ping, true);
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

                    embed
                })
            }).await?;
        },
        Err(e) => {
            error!("Failed to ping server: {}", e);

            ctx.send(|msg| {
                msg.content("Failed to ping server - is the host online?").ephemeral(true)
            }).await?;
        }
    }

    Ok(())
}
