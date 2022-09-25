mod roles;
mod diagnostics;

use roles::*;
use diagnostics::*;
use crate::{CowContext, Error};

#[poise::command(prefix_command, slash_command,
    subcommands("list", "add", "remove", "scan", "fix"),
    description_localized("en-US", "Configuration to manage ranks and levelling on the server."),
    aliases("rc"),
    identifying_name = "Rank Configuration"
)]
pub async fn rankconfig(_ctx: CowContext<'_>) -> Result<(), Error> {
    Ok(()) //list().inner(ctx).await
}