mod timeout_config;

use timeout_config::*;
use crate::{CowContext, Error};

#[poise::command(prefix_command, slash_command,
    subcommands("get", "set"),
    description_localized("en-US", "Commands for viewing and settinge the cooldown for chat xp.")
)]
pub async fn timeout(_ctx: CowContext<'_>) -> Result<(), Error> {
    Ok(()) //get().inner(ctx).await
}