use tracing::error;
use crate::{CowContext, Error};
use proto_mc::ping::ping as mc_ping;

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
    #[description = "The port of the server."] #[min = 1] #[max = 65535] port: Option<u16>)
-> Result<(), Error> {

    Ok(())
}
