use org_domain::{
    id::OrganizationId,
    organization::{
        Organization,
        value_object::{OrganizationCode, OrganizationName},
    },
};

use crate::ports::error::PortError;

#[async_trait::async_trait]
pub trait OrganizationRepository: Send + Sync {
    async fn insert(&mut self, organization: &Organization) -> Result<(), PortError>;
    async fn update(&mut self, organization: &Organization) -> Result<(), PortError>;
    async fn soft_delete(&mut self, organization: &Organization) -> Result<(), PortError>;

    async fn find_by_id(&mut self, id: &OrganizationId) -> Result<Option<Organization>, PortError>;
    async fn find_by_code(
        &mut self,
        code: &OrganizationCode,
    ) -> Result<Option<Organization>, PortError>;
    async fn find_by_name(
        &mut self,
        name: &OrganizationName,
    ) -> Result<Option<Organization>, PortError>;

    async fn exists_by_code(&mut self, code: &OrganizationCode) -> Result<bool, PortError>;
    async fn exists_by_name(&mut self, name: &OrganizationName) -> Result<bool, PortError>;

    /// 是否存在子组织（用于 Organization::ensure_deletable 前置查询，
    async fn exists_children(&mut self, parent_id: &OrganizationId) -> Result<bool, PortError>;
}
