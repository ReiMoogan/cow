pub mod rcon;
pub mod feed;
pub mod ping;

use rcon::*;
use feed::*;
use ping::*;

use crate::{CowContext, Error};

#[poise::command(prefix_command, slash_command,
    subcommands("ping", "feed", "rcon"),
    discard_spare_arguments,
    description_localized("en-US", "Fetch data from Minecraft servers."),
    aliases("mc"),
    identifying_name = "Minecraft"
)]
pub async fn minecraft(_ctx: CowContext<'_>) -> Result<(), Error> {
    Ok(())
}