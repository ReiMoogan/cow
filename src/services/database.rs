use bb8::Pool;
use bb8_tiberius::ConnectionManager;
use std::sync::Arc;
use serenity::{
    model::id::{
        UserId,
        GuildId,
        ChannelId, RoleId
    },
    prelude::TypeMapKey
};
use tiberius::{AuthMethod, Config};
use rust_decimal::{
    Decimal,
    prelude::FromPrimitive
};
use rust_decimal::prelude::ToPrimitive;
use crate::models::db_models::*;

pub struct Database {
    pub(crate) pool: Pool<ConnectionManager>
}

impl TypeMapKey for Database {
    type Value = Arc<Database>;
}

impl Database {
    pub async fn new(ip: &str, port: u16, usr: &str, pwd: &str) -> Result<Self, bb8_tiberius::Error> {
        // The password is stored in a file; using secure strings is probably not going to make much of a difference.
        let mut config = Config::new();

        config.host(ip);
        config.port(port);
        config.authentication(AuthMethod::sql_server(usr, pwd));
        // Default schema needs to be Cow
        config.database("Cow");
        config.trust_cert();

        let manager = ConnectionManager::build(config)?;
        let pool = Pool::builder().max_size(8).build(manager).await?;

        Ok(Database { pool })
    }

    pub async fn provide_exp(&self, server_id: GuildId, user_id: UserId) -> Result<LevelUp, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;
        let server = Decimal::from_u64(*server_id.as_u64()).unwrap();
        let user = Decimal::from_u64(*user_id.as_u64()).unwrap();
        let res = conn.query(
            "EXEC Ranking.ProvideExp @serverid = @P1, @userid = @P2",
            &[&server, &user])
            .await?
            .into_row()
            .await?;

        let mut out = LevelUp::new();

        if let Some(row) = res {
            let mut old_rank_id: Option<u64> = None;
            let mut new_rank_id: Option<u64> = None;

            if let Some(old_rank_id_row) = row.get(1) {
                let old_rank_id_dec: rust_decimal::Decimal = old_rank_id_row;
                old_rank_id = old_rank_id_dec.to_u64();
            }
            if let Some(new_rank_id_row) = row.get(2) {
                let new_rank_id_dec: rust_decimal::Decimal = new_rank_id_row;
                new_rank_id = new_rank_id_dec.to_u64();
            }

            out = LevelUp {
                level: row.get(0).unwrap(),
                old_rank: old_rank_id,
                new_rank: new_rank_id
            };
        }

        Ok(out)
    }

    pub async fn get_xp(&self, server_id: GuildId, user_id: UserId) -> Result<Experience, Box<dyn std::error::Error>> {
        let mut conn = self.pool.get().await?;
        let server = Decimal::from_u64(*server_id.as_u64()).unwrap();
        let user = Decimal::from_u64(*user_id.as_u64()).unwrap();
        let res = conn.query(
            "SELECT xp, level FROM [Ranking].[Level] WHERE server_id = @P1 AND [user_id] = @P2",
            &[&server, &user])
            .await?
            .into_row()
            .await?;

        let mut out = Experience::new();

        if let Some(item) = res {
            out = Experience {
                xp: item.get(0).unwrap(),
                level: item.get(1).unwrap()
            };
        }

        Ok(out)
    }

    pub async fn get_highest_role(&self, server_id: GuildId, level: i32) -> Result<Option<RoleId>, Box<dyn std::error::Error>> {
        let mut conn = self.pool.get().await?;
        let server = Decimal::from_u64(*server_id.as_u64()).unwrap();
        let res = conn.query(
            "SELECT TOP 1 role_id FROM [Ranking].[Role] WHERE server_id = @P1 AND min_level <= @P2 ORDER BY min_level DESC",
            &[&server, &level])
            .await?
            .into_row()
            .await?;

        let mut out: Option<RoleId> = None;

        if let Some(item) = res {
            let id: rust_decimal::Decimal = item.get(0).unwrap();
            out = id.to_u64().and_then(|u| Option::from(RoleId::from(u)));
        }

        Ok(out)
    }

    pub async fn calculate_level(&self, level: i32) -> Result<i32, Box<dyn std::error::Error>> {
        let mut conn = self.pool.get().await?;
        let res = conn.query(
            "EXEC [Ranking].[CalculateLevel] @level = @P1",
            &[&level])
            .await?
            .into_row()
            .await?;

        let mut out: i32 = 0;

        if let Some(item) = res {
            out = item.get(0).unwrap();
        }

        Ok(out)
    }

    // True: disabled False: enabled
    // Because by default a channel should be enabled, right?
    pub async fn toggle_channel_xp(&self, server_id: GuildId, channel_id: ChannelId) -> Result<bool, Box<dyn std::error::Error>> {
        let mut conn = self.pool.get().await?;
        let server = Decimal::from_u64(*server_id.as_u64()).unwrap();
        let channel = Decimal::from_u64(*channel_id.as_u64()).unwrap();
        let res = conn.query(
            "EXEC [Ranking].[ToggleChannel] @serverid = @P1, @channelid = @P2",
            &[&server, &channel])
            .await?
            .into_row()
            .await?;

        let mut out: bool = false;

        if let Some(item) = res {
            out = item.get(0).unwrap();
        }

        Ok(out)
    }

    // True: disabled False: enabled
    pub async fn toggle_server_ranking(&self, server_id: GuildId) -> Result<bool, Box<dyn std::error::Error>> {
        let mut conn = self.pool.get().await?;
        let server = Decimal::from_u64(*server_id.as_u64()).unwrap();
        let res = conn.query(
            "UPDATE [Ranking].[Server] SET ranking_disabled = ~ranking_disabled WHERE id = @P1;\
            SELECT ranking_disabled FROM [Ranking].[Server] WHERE id = @P1;",
            &[&server])
            .await?
            .into_row()
            .await?;

        let mut out: bool = false;

        if let Some(item) = res {
            out = item.get(0).unwrap();
        }

        Ok(out)
    }

    pub async fn get_disablements(&self, server_id: GuildId, channel_id: ChannelId) -> Result<Disablements, Box<dyn std::error::Error>> {
        let mut conn = self.pool.get().await?;
        let server = Decimal::from_u64(*server_id.as_u64()).unwrap();
        let channel = Decimal::from_u64(*channel_id.as_u64()).unwrap();

        let res = conn.query(
            "SELECT \
                (SELECT CAST(1 AS BIT) FROM [Ranking].[DisabledChannel] WHERE server_id = @P1 AND channel_id = @P2), \
                (SELECT ranking_disabled FROM [Ranking].[Server] WHERE id = @P1);",
            &[&server, &channel])
            .await?
            .into_row()
            .await?;

        let mut out: Disablements = Disablements {
            channel: false,
            guild: false
        };

        if let Some(item) = res {
            let channel_disabled: Option<bool> = item.get(0);
            let guild_disabled: Option<bool> = item.get(1);

            out = Disablements {
                channel: channel_disabled.unwrap_or_default(),
                guild: guild_disabled.unwrap_or_default()
            };
        }

        Ok(out)
    }

    // Page number is zero-indexed.
    pub async fn top_members(&self, server_id: GuildId, page: i32) -> Result<MemberPagination, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;
        let server = Decimal::from_u64(*server_id.as_u64()).unwrap();
        const ROWS_FETCHED: i32 = 10;
        let mut offset = page * ROWS_FETCHED;
        offset = offset.max(0);
        let res = conn.query(
            "SELECT user_id, level, xp FROM [Ranking].[Level] WHERE server_id = @P1 ORDER BY level DESC, xp DESC OFFSET @P2 ROWS FETCH NEXT @P3 ROWS ONLY; SELECT COUNT(1) FROM [Ranking].[Level] WHERE server_id = @P1",
            &[&server, &offset, &ROWS_FETCHED])
            .await?
            .into_results()
            .await?;

        let count: i32 = res.get(1).unwrap().first().unwrap().get(0).unwrap();

        let members = res.first().unwrap().iter()
            .map(|row| {
                let id: rust_decimal::Decimal = row.get(0).unwrap();
                Member {
                    id: UserId::from(id.to_u64().unwrap()),
                    exp: Experience {
                        level: row.get(1).unwrap(),
                        xp: row.get(2).unwrap()
                    }
                }
            })
            .collect::<Vec<_>>();

        let pages = (count / ROWS_FETCHED) + ((count % ROWS_FETCHED != 0) as i32); // Divide, then round if not perfect division

        Ok(MemberPagination {
            members,
            current_page: page,
            last_page: pages
        })
    }

    pub async fn rank_within_members(&self, server_id: GuildId, user_id: UserId) -> Result<Option<i64>, Box<dyn std::error::Error>> {
        let mut conn = self.pool.get().await?;
        let server = Decimal::from_u64(*server_id.as_u64()).unwrap();
        let user = Decimal::from_u64(*user_id.as_u64()).unwrap();
        let res = conn.query(
            "SELECT row_number FROM (SELECT user_id, ROW_NUMBER() OVER (ORDER BY level DESC, xp DESC) AS row_number FROM [Ranking].[Level] WHERE server_id = @P1) mukyu WHERE user_id = @P2",
            &[&server, &user])
            .await?
            .into_row()
            .await?;

        let mut out: Option<i64> = None;

        if let Some(item) = res {
            // Apparently it's an i64. Cool.
            out = item.get(0);
        }

        Ok(out)
    }

    pub async fn get_roles(&self, server_id: GuildId) -> Result<Vec<Rank>, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;
        let server = Decimal::from_u64(*server_id.as_u64()).unwrap();
        let res = conn.query(
            "SELECT role_name, role_id, min_level FROM [Ranking].[Role] WHERE server_id = @P1 ORDER BY min_level ASC",
            &[&server])
            .await?
            .into_first_result()
            .await?
            .into_iter()
            .map(|row| {
                let name: &str = row.get(0).unwrap();
                let mut id: Option<RoleId> = None;
                if let Some(row) = row.get(1) {
                    let id_dec: rust_decimal::Decimal = row;
                    id = id_dec.to_u64().and_then(|u| Option::from(RoleId::from(u)));
                }

                Rank {
                    name: name.to_string(),
                    role_id: id,
                    min_level: row.get(2).unwrap()
                }
            })
            .collect::<Vec<_>>();

        Ok(res)
    }

    // will also set role
    pub async fn add_role(&self, server_id: GuildId, role_name: &str, role_id: RoleId, min_level: i32) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;
        let server = Decimal::from_u64(*server_id.as_u64()).unwrap();
        let role = Decimal::from_u64(*role_id.as_u64()).unwrap();
        let res = conn.query(
            "EXEC [Ranking].[AddRole] @server_id = @P1, @role_name = @P2, @role_id = @P3, @min_level = @P4",
            &[&server, &role_name, &role, &Decimal::from_i32(min_level).unwrap()])
            .await?
            .into_row()
            .await?;

        let mut out: bool = false;

        if let Some(item) = res {
            out = item.get(0).unwrap();
        }

        Ok(out)
    }

    pub async fn remove_role(&self, server_id: GuildId, role_id: RoleId) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;
        let server = Decimal::from_u64(*server_id.as_u64()).unwrap();
        let role = Decimal::from_u64(*role_id.as_u64()).unwrap();
        let res = conn.query(
            "EXEC [Ranking].[RemoveRole] @serverid = @P1, @roleid = @P2",
            &[&server, &role])
            .await?
            .into_row()
            .await?;

        let mut out: bool = false;

        if let Some(item) = res {
            out = item.get(0).unwrap();
        }

        Ok(out)
    }

    pub async fn set_timeout(&self, server_id: GuildId, timeout: i32) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;
        let server = Decimal::from_u64(*server_id.as_u64()).unwrap();
        let timeout = Decimal::from_i32(timeout).unwrap();
        let res = conn.query(
            "EXEC [Ranking].[SetServerTimeout] @serverid = @P1, @timeout = @P2",
            &[&server, &timeout])
            .await?
            .into_row()
            .await?;

        let mut out: bool = false;

        if let Some(item) = res {
            out = item.get(0).unwrap();
        }

        Ok(out)
    }

    pub async fn get_timeout(&self, server_id: GuildId) -> Result<i32, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;
        let server = Decimal::from_u64(*server_id.as_u64()).unwrap();
        let res = conn.query(
            "SELECT TOP 1 timeout FROM [Ranking].[Server] WHERE id=@P1",
            &[&server])
            .await?
            .into_row()
            .await?;

        let mut out: i32 = -1;

        if let Some(item) = res {
            out = item.get(0).unwrap();
        }

        Ok(out)
    }

    pub async fn get_users(&self, server_id: GuildId) -> Result<Vec<FullMember>, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().await?;
        let server = Decimal::from_u64(*server_id.as_u64()).unwrap();
        let res = conn.query(
            "EXEC [Ranking].[GetAllUsers] @serverid = @P1",
            &[&server])
            .await?
            .into_first_result()
            .await?
            .into_iter()
            .map(|row| {
                let id: UserId = row.get(0).and_then(|u: rust_decimal::Decimal| u.to_u64()).map(UserId::from).unwrap();
                let role_id: Option<RoleId> = row.get(3).and_then(|u: rust_decimal::Decimal| u.to_u64()).map(RoleId::from);
                FullMember {
                    user: id,
                    exp: Experience {
                        level: row.get(1).unwrap(),
                        xp: row.get(2).unwrap()
                    },
                    role_id
                }
            })
            .collect::<Vec<_>>();

        Ok(res)
    }
}