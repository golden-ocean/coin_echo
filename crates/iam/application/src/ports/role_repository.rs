use iam_domain::{
    id::RoleId,
    role::{
        Role,
        value_object::{RoleCode, RoleName},
    },
};

use crate::ports::PortError;

#[async_trait::async_trait]
pub trait RoleRepository: Send + Sync {
    async fn insert(&mut self, role: &Role) -> Result<(), PortError>;
    async fn update(&mut self, role: &Role) -> Result<(), PortError>;
    async fn soft_delete(&mut self, role: &Role) -> Result<(), PortError>;

    async fn find_by_id(&mut self, id: &RoleId) -> Result<Option<Role>, PortError>;
    async fn find_by_code(&mut self, code: &RoleCode) -> Result<Option<Role>, PortError>;
    async fn find_by_name(&mut self, name: &RoleName) -> Result<Option<Role>, PortError>;

    async fn exists_by_code(&mut self, code: &RoleCode) -> Result<bool, PortError>;
    async fn exists_by_name(&mut self, name: &RoleName) -> Result<bool, PortError>;
}
