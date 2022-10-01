mod roles;
mod diagnostics;

use roles::*;
use diagnostics::*;
use crate::{CowContext, Error};

#[poise::command(prefix_command, slash_command,
    subcommands("list", "add", "remove", "scan", "fix"),
    discard_spare_arguments,
    description_localized("en-US", "Configuration to manage ranks and levelling on the server."),
    aliases("rc"),
    identifying_name = "Rank Configuration"
)]
pub async fn rankconfig(ctx: CowContext<'_>) -> Result<(), Error> {
    list_code(ctx).await
}