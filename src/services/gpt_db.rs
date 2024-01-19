use serenity::model::id::UserId;
use rust_decimal::{
    Decimal,
    prelude::FromPrimitive
};

use crate::Database;

impl Database {
    pub async fn has_gpt4_enabled(&self, user_id: UserId) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;
        let user = Decimal::from_u64(user_id.get()).unwrap();
        let res = conn.query(
            "SELECT gpt4_enabled FROM [Ranking].[User] WHERE id = @P1;",
            &[&user])
            .await?
            .into_row()
            .await?;

        if let Some(row) = res {
            let gpt_enabled: Option<bool> = row.get(0);
            Ok(gpt_enabled.unwrap_or_default())
        } else {
            Ok(false)
        }
    }
}