use std::borrow::Cow;

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PortError {
    // ---- 通用数据访问错误 ----
    #[error("数据不存在: {entity}")]
    NotFound { entity: &'static str },
    #[error("唯一键冲突：{entity}-{field}")]
    UniqueConflict {
        entity: &'static str,
        field: &'static str,
    },
    #[error("{entity} 存在子级，无法删除")]
    HasChildren { entity: &'static str },
    #[error("{entity} 仍有员工关联，无法删除")]
    HasMembers { entity: &'static str },

    // ---- 基础设施/转换错误 ----
    #[error("数据库值转VO失败: {field}-{value}")]
    ValueConvert { field: &'static str, value: String },
    #[error("数据库底层驱动错误")]
    Database,
    #[error("基础设施错误: {0}")]
    Infrastructure(String),
}

impl ErrorMeta for PortError {
    fn kind(&self) -> ErrorKind {
        match self {
            Self::NotFound { .. } => ErrorKind::NotFound,
            Self::UniqueConflict { .. } | Self::HasChildren { .. } | Self::HasMembers { .. } => {
                ErrorKind::Conflict
            }
            Self::ValueConvert { .. } => ErrorKind::Internal,
            Self::Database => ErrorKind::Unavailable,
            Self::Infrastructure(_) => ErrorKind::Internal,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "org.port.not_found",
            Self::UniqueConflict { .. } => "org.port.unique_conflict",
            Self::HasChildren { .. } => "org.port.has_children",
            Self::HasMembers { .. } => "org.port.has_members",
            Self::ValueConvert { .. } => "org.port.value_convert_failed",
            Self::Database => "org.port.database_error",
            Self::Infrastructure(_) => "org.port.infrastructure_error",
        }
    }

    fn detail(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::NotFound { entity } => Some(format!("{entity} 不存在").into()),
            Self::UniqueConflict { entity, field } => {
                Some(format!("{entity} 的 {field} 已存在").into())
            }
            Self::HasChildren { entity } => {
                Some(format!("{entity} 存在子级，请先处理子级后再删除").into())
            }
            Self::HasMembers { entity } => {
                Some(format!("{entity} 仍有员工关联，请先解除关联后再删除").into())
            }
            _ => None,
        }
    }

    fn fields(&self) -> Vec<FieldError> {
        match self {
            Self::UniqueConflict { field, .. } => {
                vec![FieldError::new(*field, "unique_violation")]
            }
            Self::HasChildren { .. } => vec![FieldError::new("parent_id", "has_children")],
            Self::HasMembers { .. } => vec![FieldError::new("id", "has_members")],
            _ => Vec::new(),
        }
    }
}
