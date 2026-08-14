use std::borrow::Cow;

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PortError {
    #[error("数据不存在: {entity}")]
    NotFound { entity: &'static str },
    #[error("唯一键冲突：{entity}-{field}")]
    UniqueConflict {
        entity: &'static str,
        field: &'static str,
    },
    #[error("乐观锁版本冲突，并发更新被拦截: {entity}")]
    VersionConflict { entity: &'static str },
    #[error("工号生成失败")]
    StaffNoGenerateFailed,
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
            Self::UniqueConflict { .. } | Self::VersionConflict { .. } => ErrorKind::Conflict,
            Self::StaffNoGenerateFailed => ErrorKind::Internal,
            Self::ValueConvert { .. } => ErrorKind::Internal,
            Self::Database | Self::Infrastructure(_) => ErrorKind::Unavailable,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "iam.port.not_found",
            Self::UniqueConflict { .. } => "iam.port.unique_conflict",
            Self::VersionConflict { .. } => "iam.port.version_conflict",
            Self::StaffNoGenerateFailed => "iam.port.staff_no_generate_failed",
            Self::ValueConvert { .. } => "iam.port.value_convert_failed",
            Self::Database => "iam.port.database_error",
            Self::Infrastructure(_) => "iam.port.infrastructure_error",
        }
    }

    fn detail(&self) -> Option<Cow<'_, str>> {
        // Port 层属于基础设施，detail 在 5xx 时会被传输层脱敏
        // 但对于 Conflict/NotFound 等调用方错误，可以暴露具体实体信息
        match self {
            Self::NotFound { entity } => Some(format!("{} 不存在", entity).into()),
            Self::UniqueConflict { entity, field } => {
                Some(format!("{} 的 {} 已存在", entity, field).into())
            }
            Self::VersionConflict { entity } => Some(format!("{} 已被其他操作修改", entity).into()),
            Self::ValueConvert { field, value } => {
                Some(format!("字段 {} 的值 '{}' 无法转换为领域对象", field, value).into())
            }
            // Internal / Unavailable 不暴露 detail
            _ => None,
        }
    }

    fn fields(&self) -> Vec<FieldError> {
        match self {
            Self::UniqueConflict { field, .. } => {
                vec![FieldError::new(*field, "unique_violation")]
            }
            _ => Vec::new(),
        }
    }
}
