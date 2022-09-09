use crate::{CowContext, Error};

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en", "Info about this bot.")
)]
pub async fn info(ctx: CowContext<'_>) -> Result<(), Error> {
    const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");
    let content = format!("Cow v{} - A Discord bot written by HelloAndrew and DoggySazHi", VERSION.unwrap_or("<unknown>"));

    ctx.say(content).await?;
    Ok(())
}

/// Registers or unregisters application commands in this guild or globally
#[poise::command(prefix_command, hide_in_help, owners_only)]
pub async fn register(ctx: CowContext<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;

    Ok(())
}
