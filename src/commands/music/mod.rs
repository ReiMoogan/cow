mod music_commands;

use crate::{CowContext, Error};
use music_commands::*;

#[poise::command(prefix_command, slash_command,
    subcommands("help", "join", "leave", "play", "playlist", "pause", "now_playing", "skip", "queue"),
    description_localized("en", "Commands for playing music."),
    guild_only
)]
pub async fn parent(ctx: CowContext<'_>) -> Result<(), Error> {
    help(ctx).await
}