use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Clone)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn new(name: String, email: String, role: UserRole) -> Self {
        let now = Utc::now();
        let id = Uuid::new_v4();

        Self {
            id,
            name,
            email,
            role,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Clone)]
pub enum UserRole {
    Normal = 0,
    Developer = 1,
}

impl From<u8> for UserRole {
    fn from(role: u8) -> Self {
        match role {
            0 => Self::Normal,
            1 => Self::Developer,
            _ => panic!(),
        }
    }
}
