use platform_kernel::error::{ErrorKind, ErrorMeta};

#[derive(Debug, thiserror::Error)]
pub enum PasswordError {
    /// 密码与哈希不匹配（登录失败场景，不等同于系统故障）。
    #[error("密码不正确")]
    Mismatch,

    /// 哈希过程本身失败（内存分配、参数非法等），非用户输入问题。
    #[error("密码哈希处理失败")]
    HashingFailed,

    /// 待校验的哈希字符串格式非法（如数据库里存的不是合法 PHC 字符串）。
    #[error("密码哈希格式无效")]
    InvalidHashFormat,
}

impl ErrorMeta for PasswordError {
    fn kind(&self) -> ErrorKind {
        match self {
            Self::Mismatch => ErrorKind::Unauthenticated,
            Self::HashingFailed | Self::InvalidHashFormat => ErrorKind::Internal,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Mismatch => "security.password_mismatch",
            Self::HashingFailed => "security.password_hashing_failed",
            Self::InvalidHashFormat => "security.password_hash_invalid",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mismatch_is_caller_fault_others_are_internal() {
        assert_eq!(PasswordError::Mismatch.kind(), ErrorKind::Unauthenticated);
        assert_eq!(PasswordError::HashingFailed.kind(), ErrorKind::Internal);
        assert_eq!(PasswordError::InvalidHashFormat.kind(), ErrorKind::Internal);
    }

    #[test]
    fn codes_are_unique() {
        let codes = [
            PasswordError::Mismatch.code(),
            PasswordError::HashingFailed.code(),
            PasswordError::InvalidHashFormat.code(),
        ];
        let unique: std::collections::HashSet<_> = codes.iter().collect();
        assert_eq!(unique.len(), codes.len());
    }
}
