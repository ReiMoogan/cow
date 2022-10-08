use serenity::model::id::{RoleId, UserId};

pub struct LevelUp {
    pub level: i32,
    pub old_rank: Option<u64>,
    pub new_rank: Option<u64>
}

impl LevelUp {
    pub fn new() -> Self {
        LevelUp {
            level: 0,
            old_rank: None,
            new_rank: None
        }
    }
}

pub struct Experience {
    pub level: i32,
    pub xp: i32
}

impl Experience {
    pub fn new() -> Self {
        Experience {
            level: 0,
            xp: 0
        }
    }
}

pub struct Disablements {
    pub channel: bool,
    pub guild: bool
}

pub struct Member {
    pub id: UserId,
    pub exp: Experience,
}

pub struct FullMember {
    pub user: UserId,
    pub exp: Experience,
    pub role_id: Option<RoleId>
}

pub struct Rank {
    pub name: String,
    pub role_id: Option<RoleId>,
    pub min_level: i32
}

pub struct MemberPagination {
    pub members: Vec<Member>,
    pub current_page: i32,
    pub last_page: i32
}