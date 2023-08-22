use serde_derive::Serialize;

#[derive(Serialize, Clone)]
pub struct User {
    pub username: String,
    pub cooldown: i64,
    pub score: u32,
    pub rank: u32,
    pub verified: bool,
}

impl User {
    pub fn new(username: String, score: u32, verified: bool) -> Self {
        Self {
            username,
            cooldown: 0,
            score,
            rank: 0,
            verified,
        }
    }
}