use tracing::error;
use crate::{CowContext, Error};
use super::parse_input;
use proto_mc::rcon::RCONClient;

#[poise::command(
    prefix_command,
    slash_command,
    hide_in_help,
    owners_only,
    description_localized("en-US", "Send commands to a Minecraft server."),
    discard_spare_arguments
)]
pub async fn rcon(
    ctx: CowContext<'_>,
    #[description = "The hostname of the server."] host: String,
    #[description = "The password of the server."] password: String,
    #[description = "The command to execute."] command: String,
    #[description = "The port of the server."] #[min = 1] #[max = 65535] port: Option<u16>)
-> Result<(), Error> {
    let input = parse_input(&host, port);

    match input {
        Ok(connection) => {
            ctx.defer().await?;

            let mut client = RCONClient::<&str>::new(&connection, &password);

            if let Err(ex) = client.connect().await {
                error!("Failed to connect to the server: {}", ex);

                ctx.send(|msg| {
                    msg.content("Failed to connect to the server. Is it online?").ephemeral(true)
                }).await?;
            }

            if let Err(ex) = client.login().await {
                error!("Failed to login: {}", ex);

                ctx.send(|msg| {
                    msg.content("Failed to login. Is the password correct?").ephemeral(true)
                }).await?;
            }

            match client.send(&command).await {
                Ok(result) => {
                    ctx.send(|msg| {
                        let mut str = result.payload;
                        str.truncate(2000);
                        msg.content(str).ephemeral(true)
                    }).await?;
                }
                Err(ex) => {
                    error!("Failed to execute commmand: {}", ex);

                    ctx.send(|msg| {
                        msg.content("Failed to execute command. Did the server crash?").ephemeral(true)
                    }).await?;
                }
            }
        }
        Err(ex) => {
            ctx.send(|msg| {
                msg.content(ex.to_string()).ephemeral(true)
            }).await?;
        }
    }

    Ok(())
}
