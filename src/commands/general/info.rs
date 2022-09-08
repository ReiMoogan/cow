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