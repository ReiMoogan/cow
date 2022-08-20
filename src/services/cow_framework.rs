use serenity::client::Context;
use serenity::framework::{Framework, StandardFramework};
use serenity::model::channel::Message;
use async_trait::async_trait;


pub struct CowFramework {
    internal_framework: StandardFramework
}

#[async_trait]
impl Framework for CowFramework {
    async fn dispatch(&self, ctx: Context, msg: Message) {

    }
}