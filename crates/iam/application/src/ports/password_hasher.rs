use platform_kernel::error::{ErrorKind, ErrorMeta};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PasswordHasherError {
    #[error("哈希错误")]
    Hash,
    #[error("验证错误")]
    Verify,
}

impl ErrorMeta for PasswordHasherError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Internal
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Hash => "iam.password_hash_failed",
            Self::Verify => "iam.password_verify_failed",
        }
    }
}

#[async_trait::async_trait]
pub trait PasswordHasher: Send + Sync {
    /// 明文密码生成标准 PHC 格式哈希
    async fn hash(&self, raw: &str) -> Result<String, PasswordHasherError>;
    /// 明文与PHC哈希比对
    async fn verify(&self, raw: &str, hash: &str) -> Result<(), PasswordHasherError>;
}
