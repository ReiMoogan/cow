use serenity::{
    client::Context,
    model::{
        gateway::Ready
    }
};


use tracing::{info};

pub async fn ready(_ctx: &Context, ready: &Ready) {
    info!("Logged in as {}", ready.user.name);
}