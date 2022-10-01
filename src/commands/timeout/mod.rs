mod timeout_config;

use timeout_config::*;
use crate::{CowContext, Error};

#[poise::command(prefix_command, slash_command,
    subcommands("get", "set"),
    discard_spare_arguments,
    description_localized("en-US", "Commands for viewing and settinge the cooldown for chat xp."),
    identifying_name = "Leveling Timeout"
)]
pub async fn timeout(ctx: CowContext<'_>) -> Result<(), Error> {
    get_code(ctx).await
}