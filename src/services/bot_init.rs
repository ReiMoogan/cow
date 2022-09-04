use serenity::{
    client::Context,
    model::{
        gateway::Ready,
        application::command::Command
    }
};


use log::{error, info};

async fn register_slash_commands(ctx: &Context, _: &Ready) {
    if let Err(ex) = Command::create_global_application_command(&ctx.http, |cmd| {
        cmd.name("info").description("Get information about this bot.")
    }).await {
        error!("Cannot create slash command: {}", ex)
    } else {
        info!("Finished creating slash commands.")
    }
}

pub async fn ready(ctx: &Context, ready: &Ready) {
    info!("Logged in as {}", ready.user.name);
    register_slash_commands(ctx, ready).await;
}