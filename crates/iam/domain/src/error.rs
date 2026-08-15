//! iam-domain 顶层错误聚合。

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};
use platform_kernel::meta::StatusError;

use crate::id::{PermissionId, RoleId, UserId};
use crate::permission::value_object::{
    ApiMethodError, PermissionCodeError, PermissionKindError, PermissionNameError,
};
use crate::role::value_object::{RoleCodeError, RoleNameError};
use crate::user::value_object::{
    DataScopeError, EmailError, EmploymentStatusError, GenderError, PasswordCredentialError,
    PhoneError, StaffNoError,
};

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error(transparent)]
    Status(#[from] StatusError),

    // ---- User 值对象错误 ----
    #[error(transparent)]
    UserDataScope(#[from] DataScopeError),
    #[error(transparent)]
    UserEmail(#[from] EmailError),
    #[error(transparent)]
    UserEmploymentStatus(#[from] EmploymentStatusError),
    #[error(transparent)]
    UserGender(#[from] GenderError),
    #[error(transparent)]
    UserPasswordCredential(#[from] PasswordCredentialError),
    #[error(transparent)]
    UserPhone(#[from] PhoneError),
    #[error(transparent)]
    UserStaffNo(#[from] StaffNoError),

    // ---- User 状态/权限规则 ----
    #[error("用户 {id} 已启用")]
    UserStatusAlreadyEnabled { id: UserId },
    #[error("用户 {id} 已禁用")]
    UserStatusAlreadyDisabled { id: UserId },
    #[error("用户 {id} 已被停用")]
    UserSuspended { id: UserId },
    #[error("用户 {id} 不存在")]
    UserNotFound { id: UserId },
    #[error("用户 {id} 受系统保护，禁止修改")]
    UserProtected { id: UserId },

    /// Role 相关错误
    // ---- Role 值对象错误 ----
    #[error(transparent)]
    RoleCode(#[from] RoleCodeError),
    #[error(transparent)]
    RoleName(#[from] RoleNameError),

    #[error("角色 {id} 不存在")]
    RoleNotFound { id: RoleId },
    #[error("角色 {id} 已启用")]
    RoleStatusAlreadyEnabled { id: RoleId },
    #[error("角色 {id} 已禁用")]
    RoleStatusAlreadyDisabled { id: RoleId },
    #[error("角色 {id} 受系统保护，禁止修改")]
    RoleProtected { id: RoleId },

    /// Permission 相关错误
    // ---- Permission 值对象错误 ----
    #[error(transparent)]
    PermissionName(#[from] PermissionNameError),
    #[error(transparent)]
    PermissionCode(#[from] PermissionCodeError),
    #[error(transparent)]
    PermissionKind(#[from] PermissionKindError),
    #[error(transparent)]
    PermissionApiMethod(#[from] ApiMethodError),

    #[error("权限 {id} 不存在")]
    PermissionNotFound { id: PermissionId },
    #[error("权限 {id} 受系统保护，禁止修改")]
    PermissionProtected { id: PermissionId },
    #[error("权限 {id} 已启用")]
    PermissionStatusAlreadyEnabled { id: PermissionId },
    #[error("权限 {id} 已禁用")]
    PermissionStatusAlreadyDisabled { id: PermissionId },
    #[error("权限类型 {kind} 字段不匹配：{reason}")]
    PermissionKindFieldMismatch {
        kind: &'static str,
        reason: &'static str,
    },
    #[error("权限 {id} 父级设置无效：{reason}")]
    PermissionInvalidParent {
        id: PermissionId,
        reason: &'static str,
    },
}

impl ErrorMeta for DomainError {
    fn kind(&self) -> ErrorKind {
        match self {
            // 状态
            Self::Status(e) => e.kind(),

            // User 值对象错误：委托
            Self::UserDataScope(e) => e.kind(),
            Self::UserEmail(e) => e.kind(),
            Self::UserEmploymentStatus(e) => e.kind(),
            Self::UserGender(e) => e.kind(),
            Self::UserPasswordCredential(e) => e.kind(),
            Self::UserPhone(e) => e.kind(),
            Self::UserStaffNo(e) => e.kind(),

            // User 状态/权限规则
            Self::UserStatusAlreadyEnabled { .. } | Self::UserStatusAlreadyDisabled { .. } => {
                ErrorKind::Conflict
            }
            Self::UserSuspended { .. } | Self::UserProtected { .. } => ErrorKind::Forbidden,
            Self::UserNotFound { .. } => ErrorKind::NotFound,

            // Role 值对象错误：委托
            Self::RoleCode(e) => e.kind(),
            Self::RoleName(e) => e.kind(),

            // Role 状态/权限规则
            Self::RoleStatusAlreadyEnabled { .. } | Self::RoleStatusAlreadyDisabled { .. } => {
                ErrorKind::Conflict
            }
            Self::RoleProtected { .. } => ErrorKind::Forbidden,
            Self::RoleNotFound { .. } => ErrorKind::NotFound,

            // Permission 值对象错误：委托
            Self::PermissionName(e) => e.kind(),
            Self::PermissionCode(e) => e.kind(),
            Self::PermissionKind(e) => e.kind(),
            Self::PermissionApiMethod(e) => e.kind(),

            // Permission 状态/权限规则
            Self::PermissionStatusAlreadyEnabled { .. }
            | Self::PermissionStatusAlreadyDisabled { .. } => ErrorKind::Conflict,
            Self::PermissionProtected { .. } => ErrorKind::Forbidden,
            Self::PermissionNotFound { .. } => ErrorKind::NotFound,
            Self::PermissionKindFieldMismatch { .. } | Self::PermissionInvalidParent { .. } => {
                ErrorKind::Validation
            }
        }
    }

    fn code(&self) -> &'static str {
        match self {
            // 状态
            Self::Status(e) => e.code(),

            // User
            Self::UserDataScope(e) => e.code(),
            Self::UserEmail(e) => e.code(),
            Self::UserEmploymentStatus(e) => e.code(),
            Self::UserGender(e) => e.code(),
            Self::UserPasswordCredential(e) => e.code(),
            Self::UserPhone(e) => e.code(),
            Self::UserStaffNo(e) => e.code(),

            Self::UserStatusAlreadyEnabled { .. } => "iam.user.status.already_enabled",
            Self::UserStatusAlreadyDisabled { .. } => "iam.user.status.already_disabled",
            Self::UserSuspended { .. } => "iam.user.suspended",
            Self::UserNotFound { .. } => "iam.user.not_found",
            Self::UserProtected { .. } => "iam.user.protected",

            // Role
            Self::RoleCode(e) => e.code(),
            Self::RoleName(e) => e.code(),

            Self::RoleNotFound { .. } => "iam.role.not_found",
            Self::RoleStatusAlreadyEnabled { .. } => "iam.role.status.already_enabled",
            Self::RoleStatusAlreadyDisabled { .. } => "iam.role.status.already_disabled",
            Self::RoleProtected { .. } => "iam.role.protected",

            // Permission
            Self::PermissionName(e) => e.code(),
            Self::PermissionCode(e) => e.code(),
            Self::PermissionKind(e) => e.code(),
            Self::PermissionApiMethod(e) => e.code(),

            Self::PermissionNotFound { .. } => "iam.permission.not_found",
            Self::PermissionProtected { .. } => "iam.permission.protected",
            Self::PermissionStatusAlreadyEnabled { .. } => "iam.permission.status.already_enabled",
            Self::PermissionStatusAlreadyDisabled { .. } => {
                "iam.permission.status.already_disabled"
            }
            Self::PermissionKindFieldMismatch { .. } => "iam.permission.kind_field_mismatch",
            Self::PermissionInvalidParent { .. } => "iam.permission.invalid_parent",
        }
    }

    fn detail(&self) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            Self::Status(e) => e.detail(),

            // User
            Self::UserDataScope(e) => e.detail(),
            Self::UserEmail(e) => e.detail(),
            Self::UserEmploymentStatus(e) => e.detail(),
            Self::UserGender(e) => e.detail(),
            Self::UserPasswordCredential(e) => e.detail(),
            Self::UserPhone(e) => e.detail(),
            Self::UserStaffNo(e) => e.detail(),

            // Role
            Self::RoleCode(e) => e.detail(),
            Self::RoleName(e) => e.detail(),

            // Permission
            Self::PermissionName(e) => e.detail(),
            Self::PermissionCode(e) => e.detail(),
            Self::PermissionKind(e) => e.detail(),
            Self::PermissionApiMethod(e) => e.detail(),

            _ => None,
        }
    }

    fn fields(&self) -> Vec<FieldError> {
        match self {
            // 状态
            Self::Status(e) => e.fields(),

            // User VO
            Self::UserDataScope(e) => e.fields(),
            Self::UserEmail(e) => e.fields(),
            Self::UserEmploymentStatus(e) => e.fields(),
            Self::UserGender(e) => e.fields(),
            Self::UserPasswordCredential(e) => e.fields(),
            Self::UserPhone(e) => e.fields(),
            Self::UserStaffNo(e) => e.fields(),

            // Role VO
            Self::RoleCode(e) => e.fields(),
            Self::RoleName(e) => e.fields(),

            // Permission VO
            Self::PermissionName(e) => e.fields(),
            Self::PermissionCode(e) => e.fields(),
            Self::PermissionKind(e) => e.fields(),
            Self::PermissionApiMethod(e) => e.fields(),

            _ => Vec::new(),
        }
    }
}
