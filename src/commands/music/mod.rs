mod music_commands;
mod spotify;

use crate::{CowContext, Error};
use music_commands::*;

#[poise::command(prefix_command, slash_command,
    subcommands("help", "join", "leave", "play", "playlist", "pause", "now_playing", "skip", "queue"),
    discard_spare_arguments,
    description_localized("en-US", "Commands for playing music."),
    guild_only,
    identifying_name = "Music"
)]
pub async fn music(ctx: CowContext<'_>) -> Result<(), Error> {
    help_code(ctx).await
}