mod cowboard_config;
mod cowboard_db;
mod cowboard_db_models;
pub mod cowboard_handler;

use cowboard_config::*;
use crate::{CowContext, Error};

#[poise::command(prefix_command, slash_command,
    subcommands("info", "emote", "addthreshold", "removethreshold", "channel", "webhook"),
    description_localized("en-US", "Commands for modifying how the cowboard (starboard) functions."),
    guild_only,
    identifying_name = "Cowboard"
)]
pub async fn cowboard(_ctx: CowContext<'_>) -> Result<(), Error> {
    Ok(()) //info().inner(ctx).await
}