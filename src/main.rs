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
use lavalink_rs::{LavalinkClient, gateway::LavalinkEventHandler};
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{channel::{Reaction}, gateway::{Ready, GatewayIntents}, id::{UserId, ChannelId, MessageId}, guild::Member},
    http::Http,
    prelude::TypeMapKey
};
use serenity::model::application::command::Command;
use serenity::model::application::interaction::Interaction;
use songbird::SerenityInit;
use tracing::{error, info};
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;

type Error = Box<dyn error::Error + Send + Sync>;
type CowContext<'a> = poise::Context<'a, (), Error>;

struct Handler;

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
        if let Some(mut component) = interaction.message_component() {
            if component.data.custom_id.starts_with("full_menu") {
                if let Err(ex) = component.defer(&ctx).await {
                    error!("Failed to defer component: {}", ex);
                    return;
                }

                if let Err(ex) = commands::ucm::pavilion::print_full_menu(&ctx, &component).await {
                    error!("Failed to print full menu: {}", ex);
                }
            } else if component.data.custom_id.starts_with("map_next_floor") {
                if let Err(ex) = component.defer(&ctx).await {
                    error!("Failed to defer component: {}", ex);
                    return;
                }

                if let Err(ex) = commands::ucm::map::map_next_floor(&ctx, &mut component).await {
                    error!("Failed to get next map floor: {}", ex);
                }
            }
        }
    }
}

async fn init_logger() -> std::io::Result<()> {
    let file_appender = tracing_appender::rolling::hourly("logs", "cow.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing::subscriber::set_global_default(
        fmt::Subscriber::builder()
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_span_events(fmt::format::FmtSpan::CLOSE)
            .with_ansi(true)
            .with_max_level(tracing::Level::DEBUG)
            .finish()
            .with(fmt::Layer::default().with_writer(non_blocking))
    ).expect("Failed to set global subscriber");

    const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");
    info!("Initializing cowBot v{}", VERSION.unwrap_or("<unknown>"));
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
                Err(ex) => panic!("Are we not a bot? {ex}")
            }
        },
        Err(ex) => panic!("Failed to fetch bot info: {ex}")
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
    let database = Arc::new(Database::new(&config.sql_server_ip, config.sql_server_port, &config.sql_server_username, &config.sql_server_password).await.unwrap());

    let event_handler = Handler;

    let poise = poise::Framework::builder()
        .token(&token)
        .intents(GatewayIntents::all())
        .options(framework)
        .client_settings(move |settings| {
            settings
                .register_songbird()
                .event_handler(event_handler)
                .application_id(app_id.0)
        })
        .setup(move |_ctx, _ready, _framework| {
            Box::pin(async move {
                Ok(())
            })
        })
        .build()
        .await
        .expect("Failed to create client");

    {
        let serenity = poise.client();

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
                    let mut data = serenity.data.write().await;
                    data.insert::<Lavalink>(lava_client);
                }
                Err(ex) => {
                    error!("Failed to initialize LavaLink. {}", ex);
                }
            }
        }

        {
            let mut data = serenity.data.write().await;
            data.insert::<Database>(database.clone());
        }

        // Start our reminder task and forget about it. Tokio allows us to start without await.
        #[allow(clippy::let_underscore_future)]
        let _ = tokio::task::spawn(commands::ucm::reminders::check_reminders(serenity.data.clone(), serenity.cache_and_http.clone()));

        let commands = &poise.options().commands;
        let command_builders = poise::builtins::create_application_commands(commands);
        let try_create_commands = Command::set_global_application_commands(&serenity.cache_and_http.http, |commands| {
            *commands = command_builders;
            commands
        }).await;

        if let Err(ex) = try_create_commands {
            error!("Failed to create slash commands: {}", ex);
        }
    }

    // download ucm maps and save them in this directory
    let _ = tokio::task::spawn(services::map_download::dl_and_convert_all_maps());

    if let Err(ex) = poise.start().await {
        error!("Discord bot client error: {:?}", ex);
    }

    Ok(())
}
