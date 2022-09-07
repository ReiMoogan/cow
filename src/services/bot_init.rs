use serenity::{
    client::Context,
    model::{
        gateway::Ready
    }
};


use log::{info};

pub async fn ready(ctx: &Context, ready: &Ready) {
    info!("Logged in as {}", ready.user.name);
}