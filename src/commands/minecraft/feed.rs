use crate::{CowContext, Error};

#[poise::command(
    prefix_command,
    slash_command,
    hide_in_help,
    owners_only,
    description_localized("en-US", "Set up a text feed to a Minecraft server."),
    discard_spare_arguments
)]
pub async fn feed(
    _ctx: CowContext<'_>,
    #[description = "The hostname of the server."] _host: String,
    #[description = "The port of the server."] #[min = 1] #[max = 65535] _port: Option<u16>)
-> Result<(), Error> {

    Ok(())
}
