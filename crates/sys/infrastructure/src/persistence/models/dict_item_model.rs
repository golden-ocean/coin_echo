use chrono::{DateTime, Utc};
use uuid::Uuid;

use platform_kernel::meta::{AuditMeta, Status};
use sys_application::ports::PortError;
use sys_domain::{
    dictionary::{
        DictionaryItem,
        value_object::{DictionaryItemColor, DictionaryItemLabel, DictionaryItemValue},
    },
    id::{DictionaryId, DictionaryItemId},
};

/// 数据库 `sys_dictionary_item` 表的持久化 Model (PO)
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct DictionaryItemModel {
    pub id: Uuid,
    pub dictionary_id: Uuid,
    pub label: String,
    pub value: String,
    pub color: Option<String>,
    pub is_builtin: bool,
    pub sort: i32,
    pub remark: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<Uuid>,
}

/// 传入所属 DictionaryId 与领域实体构造 Model
impl DictionaryItemModel {
    pub fn from_entity(item: &DictionaryItem) -> Self {
        Self {
            id: item.id().as_uuid(),
            dictionary_id: item.dictionary_id().as_uuid(),
            label: item.label().as_str().to_string(),
            value: item.value().as_str().to_string(),
            color: item.color().map(|c| c.as_str().to_string()),
            is_builtin: item.is_builtin(),
            sort: item.sort(),
            remark: item.remark().map(|v| v.to_string()),
            status: item.status().to_string(),
            created_at: item.audit_meta().created_at(),
            created_by: item.audit_meta().created_by(),
            updated_at: item.audit_meta().updated_at(),
            updated_by: item.audit_meta().updated_by(),
        }
    }
}

// 基于引用的转换
impl TryFrom<&DictionaryItemModel> for DictionaryItem {
    type Error = PortError;

    fn try_from(model: &DictionaryItemModel) -> Result<Self, Self::Error> {
        let id = DictionaryItemId::from_uuid(model.id);
        let dictionary_id = DictionaryId::from_uuid(model.dictionary_id);

        let label: DictionaryItemLabel =
            model
                .label
                .as_str()
                .try_into()
                .map_err(|_| PortError::ValueConvert {
                    field: "label",
                    value: model.label.clone(),
                })?;

        let value: DictionaryItemValue =
            model
                .value
                .as_str()
                .try_into()
                .map_err(|_| PortError::ValueConvert {
                    field: "value",
                    value: model.value.clone(),
                })?;

        let color: Option<DictionaryItemColor> = match &model.color {
            Some(c) => Some(c.as_str().try_into().map_err(|_| PortError::ValueConvert {
                field: "color",
                value: c.clone(),
            })?),
            None => None,
        };

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

        Ok(DictionaryItem::restore(
            id,
            dictionary_id,
            label,
            value,
            color,
            model.is_builtin,
            model.sort,
            model.remark.clone(),
            status,
            audit_meta,
        ))
    }
}

impl TryFrom<DictionaryItemModel> for DictionaryItem {
    type Error = PortError;

    #[inline]
    fn try_from(model: DictionaryItemModel) -> Result<Self, Self::Error> {
        Self::try_from(&model)
    }
}
