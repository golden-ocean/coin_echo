use chrono::{DateTime, Utc};
use uuid::Uuid;

use org_application::ports::PortError;
use org_domain::{
    id::OrganizationId,
    organization::{
        Organization,
        value_object::{OrganizationCode, OrganizationName},
    },
};
use platform_kernel::meta::{AuditMeta, DeleteMeta, Status};

/// 数据库 `org_organization` 表的持久化 Model (PO)
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct OrganizationModel {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub code: String,
    pub contact: String,
    pub phone: String,
    pub email: String,
    pub sort: i32,
    pub remark: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<Uuid>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<Uuid>,
}

impl From<&Organization> for OrganizationModel {
    fn from(org: &Organization) -> Self {
        Self {
            id: org.id().as_uuid(),
            parent_id: org.parent_id().map(|id| id.as_uuid()),
            name: org.name().as_str().to_string(),
            code: org.code().as_str().to_string(),
            contact: org.contact().to_string(),
            phone: org.phone().to_string(),
            email: org.email().to_string(),
            sort: org.sort(),
            remark: org.remark().map(|v| v.to_string()),
            status: org.status().to_string(),
            created_at: org.audit_meta().created_at(),
            created_by: org.audit_meta().created_by(),
            updated_at: org.audit_meta().updated_at(),
            updated_by: org.audit_meta().updated_by(),
            deleted_at: org.delete_meta().deleted_at(),
            deleted_by: org.delete_meta().deleted_by(),
        }
    }
}

// 基于引用的转换，避免消耗 OrganizationModel 的所有权
impl TryFrom<&OrganizationModel> for Organization {
    type Error = PortError;

    fn try_from(model: &OrganizationModel) -> Result<Self, Self::Error> {
        let id = OrganizationId::from_uuid(model.id);
        let parent_id = model.parent_id.map(OrganizationId::from_uuid);

        let name: OrganizationName =
            model
                .name
                .as_str()
                .try_into()
                .map_err(|_| PortError::ValueConvert {
                    field: "name",
                    value: model.name.clone(),
                })?;

        let code: OrganizationCode =
            model
                .code
                .as_str()
                .try_into()
                .map_err(|_| PortError::ValueConvert {
                    field: "code",
                    value: model.code.clone(),
                })?;

        let status: Status =
            model
                .status
                .as_str()
                .try_into()
                .map_err(|_| PortError::ValueConvert {
                    field: "status",
                    value: model.status.clone(),
                })?;

        let audit_meta = AuditMeta::restore(
            model.created_at,
            model.updated_at,
            model.created_by,
            model.updated_by,
        );
        let delete_meta = DeleteMeta::restore(model.deleted_at, model.deleted_by);

        Ok(Organization::restore(
            id,
            parent_id,
            name,
            code,
            model.contact.clone(),
            model.phone.clone(),
            model.email.clone(),
            model.sort,
            model.remark.clone(),
            status,
            audit_meta,
            delete_meta,
        ))
    }
}

impl TryFrom<OrganizationModel> for Organization {
    type Error = PortError;
    #[inline]
    fn try_from(model: OrganizationModel) -> Result<Self, Self::Error> {
        Self::try_from(&model)
    }
}
