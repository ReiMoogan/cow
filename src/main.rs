mod models;
mod commands;
mod services;
mod util;

use std::collections::{HashSet};
use commands::{get_framework};
use models::config::Config;
use std::fs;
use std::sync::Arc;
use std::env;
use env_logger::Env;
use serenity::{
    async_trait,
    client::{Client, Context, EventHandler, bridge::gateway::GatewayIntents},
    model::{channel::{Message, Reaction}, gateway::Ready, interactions::Interaction, id::{UserId, GuildId, ChannelId, MessageId}, guild::Member},
    http::Http,
    framework::Framework,
    prelude::TypeMapKey
};
use log::{error, info};

async fn init_logger() -> std::io::Result<()> {
    let env = Env::default().default_filter_or("warning");
    env_logger::init_from_env(env);

    const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");
    info!("Initializing cow v{}", VERSION.unwrap_or("<unknown>"));
    info!("Reading from {}", env::current_dir()?.display());

    Ok(())
}

async fn fetch_bot_info(token: &str) -> (UserId, HashSet<UserId>) {
    let http = Http::new_with_token(token);

    let (app_id, owners) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();

            if let Some(team) = info.team {
                owners.insert(team.owner_user_id);
            } else {
                owners.insert(info.owner.id);
            }

            match http.get_current_user().await {
                Ok(app_id) => (app_id.id, owners),
                Err(ex) => panic!("Are we not a bot? {}", ex)
            }
        },
        Err(ex) => panic!("Failed to fetch bot info: {}", ex)
    };

    (app_id, owners)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>  {
    if let Err(ex) = init_logger().await {
        error!("Failed to initialize logger: {}", ex);
    }

    let config_json = fs::read_to_string("config.json").expect("config.json not found");
    let config : Config = serde_json::from_str(&config_json).expect("config.json is malformed");

    let token = config.token;
    let (app_id, owners) = fetch_bot_info(&token).await;
    let framework = get_framework(&config.cmd_prefix, app_id, owners).await;
    let db_clone = event_handler.database.clone();

    let mut client = Client::builder(&token)
        .event_handler(event_handler)
        .application_id(*app_id.as_u64())
        .framework_arc(framework)
        .intents(GatewayIntents::all())
        .await
        .expect("Discord failed to initialize");

    if let Err(ex) = client.start().await {
        error!("Discord bot client error: {:?}", ex);
    }

    Ok(())
}
