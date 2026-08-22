use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};
use platform_kernel::meta::StatusError;

use crate::id::{OrganizationId, PositionId};
use crate::organization::value_object::{OrganizationCodeError, OrganizationNameError};
use crate::position::value_object::{PositionCodeError, PositionNameError};

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error(transparent)]
    Status(#[from] StatusError),

    // ---- Organization ----
    #[error(transparent)]
    OrganizationCode(#[from] OrganizationCodeError),
    #[error(transparent)]
    OrganizationName(#[from] OrganizationNameError),

    #[error("组织 {id} 还有子部门")]
    OrganizationHasChildren { id: OrganizationId },
    #[error("组织 {id} 已启用")]
    OrganizationStatusAlreadyEnabled { id: OrganizationId },
    #[error("组织 {id} 已禁用")]
    OrganizationStatusAlreadyDisabled { id: OrganizationId },
    #[error("组织 {id} 不存在")]
    OrganizationNotFound { id: OrganizationId },
    #[error("组织 {id} 无效的父部门")]
    OrganizationInvalidParent { id: OrganizationId },
    #[error("组织 {id} 还有员工")]
    OrganizationHasMembers { id: OrganizationId },

    // ---- Position ----
    #[error(transparent)]
    PositionCode(#[from] PositionCodeError),
    #[error(transparent)]
    PositionName(#[from] PositionNameError),

    #[error("职位 {id} 不存在")]
    PositionNotFound { id: PositionId },
    #[error("职位 {id} 已启用")]
    PositionStatusAlreadyEnabled { id: PositionId },
    #[error("职位 {id} 已禁用")]
    PositionStatusAlreadyDisabled { id: PositionId },
    #[error("职位 {id} 还有成员")]
    PositionHasMembers { id: PositionId },
}

impl ErrorMeta for DomainError {
    fn kind(&self) -> ErrorKind {
        match self {
            // 状态
            Self::Status(e) => e.kind(),

            // Organization
            Self::OrganizationCode(e) => e.kind(),
            Self::OrganizationName(e) => e.kind(),

            Self::OrganizationHasChildren { .. }
            | Self::OrganizationHasMembers { .. }
            | Self::OrganizationInvalidParent { .. } => ErrorKind::Validation,
            Self::OrganizationStatusAlreadyEnabled { .. }
            | Self::OrganizationStatusAlreadyDisabled { .. } => ErrorKind::Conflict,
            Self::OrganizationNotFound { .. } => ErrorKind::NotFound,

            // Position
            Self::PositionCode(e) => e.kind(),
            Self::PositionName(e) => e.kind(),

            Self::PositionNotFound { .. } => ErrorKind::NotFound,
            Self::PositionStatusAlreadyEnabled { .. }
            | Self::PositionStatusAlreadyDisabled { .. } => ErrorKind::Conflict,
            Self::PositionHasMembers { .. } => ErrorKind::Validation,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            // 状态
            Self::Status(e) => e.code(),

            // Organization
            Self::OrganizationCode(e) => e.code(),
            Self::OrganizationName(e) => e.code(),

            Self::OrganizationHasChildren { .. } => "org.organization.has_children",
            Self::OrganizationHasMembers { .. } => "org.organization.has_members",
            Self::OrganizationInvalidParent { .. } => "org.organization.invalid_parent",
            Self::OrganizationStatusAlreadyEnabled { .. } => {
                "org.organization.status.already_enabled"
            }
            Self::OrganizationStatusAlreadyDisabled { .. } => {
                "org.organization.status.already_disabled"
            }
            Self::OrganizationNotFound { .. } => "org.organization.not_found",

            // Position
            Self::PositionCode(e) => e.code(),
            Self::PositionName(e) => e.code(),

            Self::PositionNotFound { .. } => "org.position.not_found",
            Self::PositionStatusAlreadyEnabled { .. } => "org.position.status.already_enabled",
            Self::PositionStatusAlreadyDisabled { .. } => "org.position.status.already_disabled",
            Self::PositionHasMembers { .. } => "org.position.has_members",
        }
    }

    fn detail(&self) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            Self::Status(e) => e.detail(),

            // Organization
            Self::OrganizationCode(e) => e.detail(),
            Self::OrganizationName(e) => e.detail(),

            _ => None,
        }
    }

    fn fields(&self) -> Vec<FieldError> {
        match self {
            // 状态
            Self::Status(e) => e.fields(),

            // Organization VO
            Self::OrganizationCode(e) => e.fields(),
            Self::OrganizationName(e) => e.fields(),

            _ => Vec::new(),
        }
    }
}
