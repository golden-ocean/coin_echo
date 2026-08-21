use chrono::{DateTime, Utc};
use uuid::Uuid;

use platform_kernel::meta::{AuditMeta, Status};
use sys_application::ports::PortError;
use sys_domain::{
    dictionary::{
        Dictionary,
        value_object::{DictionaryCode, DictionaryName},
    },
    id::DictionaryId,
};

/// 数据库 `sys_dictionary` 表的持久化 Model (PO)
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct DictionaryModel {
    pub id: Uuid,
    pub name: String,
    pub code: String,
    pub is_builtin: bool,
    pub sort: i32,
    pub remark: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<Uuid>,
}

impl From<&Dictionary> for DictionaryModel {
    fn from(dictionary: &Dictionary) -> Self {
        Self {
            id: dictionary.id().as_uuid(),
            name: dictionary.name().as_str().to_string(),
            code: dictionary.code().as_str().to_string(),
            is_builtin: dictionary.is_builtin(),
            sort: dictionary.sort(),
            remark: dictionary.remark().map(|v| v.to_string()),
            status: dictionary.status().to_string(),
            created_at: dictionary.audit_meta().created_at(),
            created_by: dictionary.audit_meta().created_by(),
            updated_at: dictionary.audit_meta().updated_at(),
            updated_by: dictionary.audit_meta().updated_by(),
        }
    }
}

// 基于引用的转换，避免消耗 DictionaryModel 的所有权
impl TryFrom<&DictionaryModel> for Dictionary {
    type Error = PortError;

    fn try_from(model: &DictionaryModel) -> Result<Self, Self::Error> {
        let id = DictionaryId::from_uuid(model.id);

        let name: DictionaryName =
            model
                .name
                .as_str()
                .try_into()
                .map_err(|_| PortError::ValueConvert {
                    field: "name",
                    value: model.name.clone(),
                })?;

        let code: DictionaryCode =
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

        Ok(Dictionary::restore(
            id,
            name,
            code,
            model.is_builtin,
            model.sort,
            model.remark.clone(),
            status,
            audit_meta,
        ))
    }
}

impl TryFrom<DictionaryModel> for Dictionary {
    type Error = PortError;

    #[inline]
    fn try_from(model: DictionaryModel) -> Result<Self, Self::Error> {
        Self::try_from(&model)
    }
}
