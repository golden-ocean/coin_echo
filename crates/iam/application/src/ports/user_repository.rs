use iam_domain::{
    id::UserId,
    user::{
        User,
        value_object::{Email, Phone},
    },
};

use crate::ports::error::PortError;

#[async_trait::async_trait]
pub trait UserRepository: Send + Sync {
    async fn insert(&mut self, user: &User) -> Result<(), PortError>;
    async fn update(&mut self, user: &User) -> Result<(), PortError>;
    async fn soft_delete(&mut self, user: &User) -> Result<(), PortError>;

    async fn find_by_id(&mut self, user_id: &UserId) -> Result<Option<User>, PortError>;
    async fn find_by_username(&mut self, username: &str) -> Result<Option<User>, PortError>;
    async fn find_by_email(&mut self, email: &Email) -> Result<Option<User>, PortError>;
    async fn find_by_phone(&mut self, phone: &Phone) -> Result<Option<User>, PortError>;

    async fn exists_by_username(&mut self, username: &str) -> Result<bool, PortError>;
    async fn exists_by_email(&mut self, email: &Email) -> Result<bool, PortError>;
    async fn exists_by_phone(&mut self, phone: &Phone) -> Result<bool, PortError>;
}
