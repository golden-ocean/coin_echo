use platform_kernel::error::{ErrorKind, ErrorMeta};

#[derive(Debug, thiserror::Error)]
pub enum CasbinError {
    /// 权限不足：策略明确拒绝该操作。
    #[error("权限不足")]
    PermissionDenied,

    /// enforcer 初始化失败（model/policy 文件缺失或格式错误）。
    #[error("Casbin Enforcer 初始化失败：{0}")]
    InitFailed(String),

    /// 策略读写失败（如 adapter 层的存储错误）。
    #[error("策略操作失败：{0}")]
    PolicyOperationFailed(String),
}

impl ErrorMeta for CasbinError {
    fn kind(&self) -> ErrorKind {
        match self {
            Self::PermissionDenied => ErrorKind::Forbidden,
            Self::InitFailed(_) | Self::PolicyOperationFailed(_) => ErrorKind::Internal,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::PermissionDenied => "security.permission_denied",
            Self::InitFailed(_) => "security.casbin_init_failed",
            Self::PolicyOperationFailed(_) => "security.casbin_policy_op_failed",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_denied_is_forbidden_not_internal() {
        assert_eq!(CasbinError::PermissionDenied.kind(), ErrorKind::Forbidden);
        assert!(CasbinError::PermissionDenied.kind().is_caller_fault());
    }

    #[test]
    fn init_and_policy_errors_are_internal() {
        assert_eq!(
            CasbinError::InitFailed("x".into()).kind(),
            ErrorKind::Internal
        );
        assert_eq!(
            CasbinError::PolicyOperationFailed("x".into()).kind(),
            ErrorKind::Internal
        );
    }
}
