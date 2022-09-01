mod models;
mod commands;
mod services;
mod util;

use std::collections::{HashSet};
use commands::{get_framework};
use models::config::Config;
use services::{*, database::Database};
use std::fs;
use std::sync::Arc;
use std::env;
use std::error;
use env_logger::Env;
use lavalink_rs::{LavalinkClient, gateway::LavalinkEventHandler};
use serenity::{
    async_trait,
    client::{Client, Context, EventHandler},
    model::{channel::{Reaction}, gateway::{Ready, GatewayIntents}, application::{interaction::Interaction}, id::{UserId, ChannelId, MessageId}, guild::Member},
    http::Http,
    framework::Framework,
    prelude::TypeMapKey
};
use log::{error, info};
use songbird::SerenityInit;

struct Handler {
    framework: Arc<Box<dyn Framework + Sync + Send>>,
    database: Arc<Database>
}

struct Lavalink;

impl TypeMapKey for Lavalink {
    type Value = LavalinkClient;
}

struct LavalinkHandler;

#[async_trait]
impl LavalinkEventHandler for LavalinkHandler { }

#[async_trait]
impl EventHandler for Handler {
    async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
        message_handler::on_join(&ctx, &new_member).await;
    }

    async fn reaction_add(&self, ctx: Context, added_reaction: Reaction) {
        commands::cowboard::cowboard_handler::add_reaction(&ctx, &added_reaction).await;
    }

    async fn reaction_remove(&self, ctx: Context, removed_reaction: Reaction) {
        commands::cowboard::cowboard_handler::remove_reaction(&ctx, &removed_reaction).await;
    }

    async fn reaction_remove_all(&self, ctx: Context, channel_id: ChannelId, removed_from_message_id: MessageId) {
        commands::cowboard::cowboard_handler::reaction_remove_all(&ctx, channel_id, removed_from_message_id).await;
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        bot_init::ready(&ctx, &ready).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        interaction_handler::interaction(&ctx, &interaction, &self.framework).await;
    }
}

async fn init_logger() -> std::io::Result<()> {
    let env = Env::default().default_filter_or("warning");
    env_logger::init_from_env(env);

    const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");
    info!("Initializing cow v{}", VERSION.unwrap_or("<unknown>"));
    info!("Reading from {}", env::current_dir()?.display());

    Ok(())
}

async fn fetch_bot_info(token: &str) -> (UserId, HashSet<UserId>) {
    let http = Http::new(token);

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
async fn main() -> Result<(), Box<dyn error::Error>>  {
    if let Err(ex) = init_logger().await {
        error!("Failed to initialize logger: {}", ex);
    }

    let config_json = fs::read_to_string("config.json").expect("config.json not found");
    let config : Config = serde_json::from_str(&config_json).expect("config.json is malformed");

    let token = config.token;
    let (app_id, owners) = fetch_bot_info(&token).await;
    let framework = get_framework(&config.cmd_prefix, app_id, owners).await;

    let event_handler = Handler {
        framework: framework.clone(),
        database: Arc::new(Database::new(&*config.sql_server_ip, config.sql_server_port, &*config.sql_server_username, &*config.sql_server_password).await.unwrap())
    };

    let db_clone = event_handler.database.clone();

    let mut client = Client::builder(&token, GatewayIntents::all())
        .event_handler(event_handler)
        .application_id(*app_id.as_u64())
        .framework_arc(framework)
        .register_songbird()
        .await
        .expect("Discord failed to initialize");

    let lavalink_enabled = !config.lavalink_ip.is_empty() && !config.lavalink_password.is_empty();

    if lavalink_enabled {
        match LavalinkClient::builder(*app_id.as_u64())
            .set_host(config.lavalink_ip)
            .set_password(
                config.lavalink_password,
            )
            .build(LavalinkHandler)
            .await {
            Ok(lava_client) => {
                let mut data = client.data.write().await;
                data.insert::<Lavalink>(lava_client);
            }
            Err(ex) => {
                error!("Failed to initialize LavaLink. {}", ex);
            }
        }
    }

    {
        let mut data = client.data.write().await;
        // Should I wrap it with an RwLock? ...it's pooled and async is nice, but...
        data.insert::<Database>(db_clone);
    }

    // Start our reminder task and forget about it.
    let _ = tokio::task::spawn(commands::ucm::reminders::check_reminders(client.data.clone(), client.cache_and_http.clone()));

    if let Err(ex) = client.start().await {
        error!("Discord bot client error: {:?}", ex);
    }

    Ok(())
}