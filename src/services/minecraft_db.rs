use serenity::model::id::ChannelId;
use rust_decimal::{
    Decimal,
    prelude::FromPrimitive
};

use crate::Database;
use crate::models::minecraft_db_models::*;

impl Database {
    pub async fn get_minecraft_channel(&self, channel_id: ChannelId) -> Result<Option<Feed>, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;
        let server = Decimal::from_u64(*channel_id.as_u64()).unwrap();
        let res = conn.query(
            "SELECT host, password FROM [Minecraft].[Feed] WHERE channel_id = @P1;",
            &[&server])
            .await?
            .into_row()
            .await?;

        Ok(res.map(|row| {
            let host: Option<&str> = row.get(0);
            let password: Option<&str> = row.get(1);

            Feed {
                channel: channel_id,
                host: host.unwrap().to_string(),
                password: password.unwrap().to_string()
            }
        }))
    }
}