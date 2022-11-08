use sysinfo::{CpuExt, System, SystemExt};
use crate::{CowContext, Error};

#[poise::command(
    prefix_command,
    slash_command,
    description_localized("en-US", "Info about this bot."),
    discard_spare_arguments
)]
pub async fn info(ctx: CowContext<'_>) -> Result<(), Error> {
    const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");

    let mut sys = System::new();
    sys.refresh_cpu();
    sys.refresh_memory();
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    sys.refresh_cpu(); // Twice to get a CPU reading.
    let uptime = sys.uptime();

    let message = format!("\
    Cow v{} - A Discord bot written by HelloAndrew and DoggySazHi \n\
    ```\
    Server: {}\n\
    System uptime: {}:{:0>2}:{:0>2}:{:0>2} \n\n\
    CPU: {:.2}% \n\
    Memory: {:?}/{:?} MiB \n\
    Swap: {:?}/{:?} MiB \n\
    ```",
    VERSION.unwrap_or("<unknown>"),
    sys.host_name().unwrap_or_default(),
    uptime / 60 / 60 / 24, (uptime / 60 / 60) % 24, (uptime / 60) % 60, uptime % 60,
    sys.global_cpu_info().cpu_usage(),
    sys.used_memory() / 1024 / 1024, sys.total_memory() / 1024 / 1024,
    sys.used_swap() / 1024 / 1024, sys.total_swap() / 1024 / 1024);

    ctx.say(message).await?;
    Ok(())
}

/// Registers or unregisters application commands in this guild or globally
#[poise::command(prefix_command, hide_in_help, owners_only, discard_spare_arguments)]
pub async fn register(ctx: CowContext<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;

    Ok(())
}
