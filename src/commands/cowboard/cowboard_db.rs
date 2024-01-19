use serenity::{
    model::id::{
        GuildId,
        ChannelId
    }
};
use rust_decimal::{
    Decimal,
    prelude::FromPrimitive
};
use rust_decimal::prelude::ToPrimitive;
use serenity::model::id::MessageId;

use crate::Database;
use crate::commands::cowboard::cowboard_db_models::*;

// Separating the database into different modules so it doesn't become a 2000 line file.
impl Database {
    pub async fn get_cowboard_config(&self, server_id: GuildId) -> Result<Cowboard, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;
        let server = Decimal::from_u64(server_id.get()).unwrap();
        let res = conn.query(
            "SELECT channel, add_threshold, remove_threshold, emote, webhook_id, webhook_token FROM [Cowboard].[Server] WHERE id = @P1",
            &[&server])
            .await?
            .into_row()
            .await?;

        let mut out = Cowboard::new(server_id.get());

        if let Some(item) = res {
            let channel_id: Option<Decimal> = item.get(0);
            let emote_str: &str = item.get(3).unwrap();
            let webhook_id: Option<Decimal> = item.get(4);
            let webhook_token: Option<&str> = item.get(5);
            out = Cowboard {
                id: server_id.get(),
                channel: channel_id.and_then(|o| o.to_u64()),
                add_threshold: item.get(1).unwrap(),
                remove_threshold: item.get(2).unwrap(),
                emote: emote_str.to_string(),
                webhook_id: webhook_id.and_then(|o| o.to_u64()),
                webhook_token: webhook_token.map(|o| o.to_string())
            };
        }

        Ok(out)
    }

    pub async fn update_cowboard(&self, config: &Cowboard) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;
        let server = Decimal::from_u64(config.id).unwrap();
        let channel = config.channel.map(|o| Decimal::from_u64(o).unwrap());
        let webhook_id = config.webhook_id.map(|o| Decimal::from_u64(o).unwrap());

        conn.query(
            "EXEC [Cowboard].[UpdateServer] @id = @P1, @channel = @P2, @add_threshold = @P3, @remove_threshold = @P4, @emote = @P5, @webhook_id = @P6, @webhook_token = @P7",
            &[&server, &channel, &config.add_threshold, &config.remove_threshold, &config.emote, &webhook_id, &config.webhook_token])
            .await?;

        Ok(())
    }

    pub async fn get_cowboard_message(&self, message: MessageId, channel: ChannelId, guild: GuildId) -> Result<Option<CowboardMessage>, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;
        let message_decimal = Decimal::from_u64(message.get()).unwrap();
        let channel_decimal = Decimal::from_u64(channel.get()).unwrap();
        let server_decimal = Decimal::from_u64(guild.get()).unwrap();
        let res = conn.query(
            "SELECT post_id, post_channel_id FROM [Cowboard].[Message] WHERE message_id = @P1 AND message_channel_id = @P2 AND guild_id = @P3",
            &[&message_decimal, &channel_decimal, &server_decimal])
            .await?
            .into_row()
            .await?;

        let mut out: Option<CowboardMessage> = None;

        if let Some(item) = res {
            let post_id = item.get(0).and_then(|u: Decimal| u.to_u64()).unwrap();
            let post_channel_id = item.get(1).and_then(|u: Decimal| u.to_u64()).unwrap();

            out = Some(CowboardMessage {
                message_id: message.get(),
                message_channel_id: channel.get(),
                post_id,
                post_channel_id,
                guild_id: guild.get()
            });
        }

        Ok(out)
    }

    pub async fn moo_message(&self, message: MessageId, channel: ChannelId, post_message: MessageId, post_channel: ChannelId, guild: GuildId) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;
        let message = Decimal::from_u64(message.get()).unwrap();
        let channel = Decimal::from_u64(channel.get()).unwrap();
        let post_message = Decimal::from_u64(post_message.get()).unwrap();
        let post_channel = Decimal::from_u64(post_channel.get()).unwrap();
        let server = Decimal::from_u64(guild.get()).unwrap();

        conn.query(
            "INSERT INTO [Cowboard].[Message] (message_id, message_channel_id, post_id, post_channel_id, guild_id) VALUES (@P1, @P2, @P3, @P4, @P5)",
            &[&message, &channel, &post_message, &post_channel, &server])
            .await?;

        Ok(())
    }

    pub async fn unmoo_message(&self, message: MessageId, channel: ChannelId, guild: GuildId) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;
        let message = Decimal::from_u64(message.get()).unwrap();
        let channel = Decimal::from_u64(channel.get()).unwrap();
        let server = Decimal::from_u64(guild.get()).unwrap();

        conn.query(
            "DELETE FROM [Cowboard].[Message] WHERE message_id = @P1 AND message_channel_id = @P2 AND guild_id = @P3",
            &[&message, &channel, &server])
            .await?;

        Ok(())
    }
}