use chrono::{DateTime, Utc};
use uuid::Uuid;

use org_application::ports::PortError;
use org_domain::{
    id::PositionId,
    position::{
        Position,
        value_object::{PositionCode, PositionName},
    },
};
use platform_kernel::meta::{AuditMeta, DeleteMeta, Status};

/// 数据库 `org_position` 表的持久化 Model (PO)
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct PositionModel {
    pub id: Uuid,
    pub name: String,
    pub code: String,
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

impl From<&Position> for PositionModel {
    fn from(position: &Position) -> Self {
        Self {
            id: position.id().as_uuid(),
            name: position.name().as_str().to_string(),
            code: position.code().as_str().to_string(),
            sort: position.sort(),
            remark: position.remark().map(|v| v.to_string()),
            status: position.status().to_string(),
            created_at: position.audit_meta().created_at(),
            created_by: position.audit_meta().created_by(),
            updated_at: position.audit_meta().updated_at(),
            updated_by: position.audit_meta().updated_by(),
            deleted_at: position.delete_meta().deleted_at(),
            deleted_by: position.delete_meta().deleted_by(),
        }
    }
}

impl TryFrom<&PositionModel> for Position {
    type Error = PortError;

    fn try_from(model: &PositionModel) -> Result<Self, Self::Error> {
        let id = PositionId::from_uuid(model.id);

        let name: PositionName =
            model
                .name
                .as_str()
                .try_into()
                .map_err(|_| PortError::ValueConvert {
                    field: "name",
                    value: model.name.clone(),
                })?;

        let code: PositionCode =
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

        Ok(Position::restore(
            id,
            name,
            code,
            model.sort,
            model.remark.clone(),
            status,
            audit_meta,
            delete_meta,
        ))
    }
}

impl TryFrom<PositionModel> for Position {
    type Error = PortError;
    #[inline]
    fn try_from(model: PositionModel) -> Result<Self, Self::Error> {
        Self::try_from(&model)
    }
}
