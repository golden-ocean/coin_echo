//! iam-domain 顶层错误聚合。

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};
use platform_kernel::meta::StatusError;

use crate::id::{RoleId, UserId};
use crate::role::value_object::{RoleCodeError, RoleNameError};
use crate::user::value_object::{
    DataScopeError, EmailError, EmploymentStatusError, GenderError, PasswordCredentialError,
    PhoneError, StaffNoError,
};

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error(transparent)]
    Status(#[from] StatusError),
    // ---- User 字段格式校验----
    #[error("数据权限范围枚举值错误: {0}")]
    UserDataScope(#[from] DataScopeError),
    #[error("邮箱错误, {0}")]
    UserEmail(#[from] EmailError),
    #[error("在职状态枚举值错误: {0}")]
    UserEmploymentStatus(#[from] EmploymentStatusError),
    #[error("性别枚举值错误: {0}")]
    UserGender(#[from] GenderError),
    #[error("密码哈希错误: {0}")]
    UserPasswordCredential(#[from] PasswordCredentialError),
    #[error("手机号码错误: {0}")]
    UserPhone(#[from] PhoneError),
    #[error("员工工号错误: {0}")]
    UserStaffNo(#[from] StaffNoError),
    // ---- 密码：登录/改密场景下的直接错误，无值对象承载 ----
    // #[error("哈希密码格式无效")]
    // InvalidPassword,
    // #[error("密码哈希失败")]
    // PasswordHashError,
    // #[error("密码验证失败")]
    // PasswordVerifyError,
    // #[error("距上次修改不足 {min_hours} 小时，暂不能修改密码")]
    // CoolingPeriodPassword { min_hours: i64 },
    // ---- User 状态/权限规则 ----
    #[error("用户状态已启用: {id}")]
    UserStatusAlreadyEnabled { id: UserId },
    #[error("用户状态已禁用: {id}")]
    UserStatusAlreadyDisabled { id: UserId },
    #[error("账户已被停用: {id}")]
    UserSuspended { id: UserId },
    #[error("用户不存在: {id}")]
    UserNotFound { id: UserId },
    #[error("系统内置账户受保护: {id}")]
    UserProtected { id: UserId },

    /// Role 相关错误
    // ---- Value Object 错误 ----
    #[error("角色代码错误: {0}")]
    RoleCode(#[from] RoleCodeError),
    #[error("角色名称错误: {0}")]
    RoleName(#[from] RoleNameError),

    #[error("角色不存在: {id}")]
    RoleNotFound { id: RoleId },
    #[error("角色状态已启用: {id}")]
    RoleStatusAlreadyEnabled { id: RoleId },
    #[error("角色状态已禁用: {id}")]
    RoleStatusAlreadyDisabled { id: RoleId },
    #[error("系统内置角色受保护: {id}")]
    RoleProtected { id: RoleId },
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

            Self::UserStatusAlreadyEnabled { .. } => "iam.user.status_already_enabled",
            Self::UserStatusAlreadyDisabled { .. } => "iam.user.status_already_disabled",
            Self::UserSuspended { .. } => "iam.user.suspended",
            Self::UserNotFound { .. } => "iam.user.not_found",
            Self::UserProtected { .. } => "iam.user.system_resource_protected",

            // Role
            Self::RoleCode(e) => e.code(),
            Self::RoleName(e) => e.code(),

            Self::RoleNotFound { .. } => "iam.role.not_found",
            Self::RoleStatusAlreadyEnabled { .. } => "iam.role.status_already_enabled",
            Self::RoleStatusAlreadyDisabled { .. } => "iam.role.status_already_disabled",
            Self::RoleProtected { .. } => "iam.role.system_resource_protected",
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

            _ => Vec::new(),
        }
    }
}
