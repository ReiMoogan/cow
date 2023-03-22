mod ask;

use crate::{CowContext, Error};
use ask::*;

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en-US", "Use GPT commands from Discord."),
    subcommands("ask"),
    discard_spare_arguments,
    aliases("chatgpt"),
    identifying_name = "GPT"
)]
pub async fn gpt(_ctx: CowContext<'_>) -> Result<(), Error> {
    Ok(())
}