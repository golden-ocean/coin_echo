use async_trait::async_trait;
use std::sync::Arc;

use iam_application::ports::{
    PasswordHasher as ApplicationPasswordHasher, password_hasher::PasswordHasherError,
};

use platform_security::password::{
    PasswordConfig, PasswordError as PlatformPasswordError,
    PasswordHasher as PlatformPasswordHasher,
};

/// 基于 Argon2id 的密码哈希适配器
///
/// 内部包裹 `platform_security::PasswordHasher`，并将 CPU 密集的哈希/校验计算
/// 卸载（Offload）到 Tokio 的 blocking 线程池中，避免阻塞主异步 Runtime 线程。
#[derive(Debug, Clone)]
pub struct Argon2PasswordHasher {
    inner: Arc<PlatformPasswordHasher>,
}

impl Argon2PasswordHasher {
    /// 通过传入已准备好的 `PlatformPasswordHasher` 创建实例
    pub fn new(hasher: PlatformPasswordHasher) -> Self {
        Self {
            inner: Arc::new(hasher),
        }
    }

    /// 通过 `PasswordConfig` 配置直接构建实例
    pub fn from_config(config: &PasswordConfig) -> Result<Self, PlatformPasswordError> {
        let hasher = PlatformPasswordHasher::new(config)?;
        Ok(Self::new(hasher))
    }
}

#[async_trait]
impl ApplicationPasswordHasher for Argon2PasswordHasher {
    async fn hash(&self, raw: &str) -> Result<String, PasswordHasherError> {
        let inner = Arc::clone(&self.inner);
        let raw = raw.to_string();

        // 将 CPU 耗时的 Argon2 计算扔给 blocking 线程池
        tokio::task::spawn_blocking(move || inner.hash(&raw))
            .await
            // 处理 JoinError（如线程池 Task 被取消/Panic）
            .map_err(|_| PasswordHasherError::Hash)?
            // 将 platform_security 的 Error 转换映射为 Application 层的 Error
            .map_err(|_| PasswordHasherError::Hash)
    }

    async fn verify(&self, raw: &str, hash: &str) -> Result<(), PasswordHasherError> {
        let inner = Arc::clone(&self.inner);
        let raw = raw.to_string();
        let hash = hash.to_string();

        tokio::task::spawn_blocking(move || inner.verify(&raw, &hash))
            .await
            .map_err(|_| PasswordHasherError::Verify)?
            .map_err(|err| match err {
                // 无论是密码不匹配，还是 PHC 字符串格式非法，应用层校验端口统归为 Verify 错误
                PlatformPasswordError::Mismatch => PasswordHasherError::Verify,
                PlatformPasswordError::InvalidHashFormat => PasswordHasherError::Verify,
                PlatformPasswordError::HashingFailed => PasswordHasherError::Verify,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_adapter() -> Argon2PasswordHasher {
        let config = PasswordConfig::default();
        Argon2PasswordHasher::from_config(&config).expect("测试配置初始化不应失败")
    }

    #[tokio::test]
    async fn test_hash_and_verify_success() {
        let adapter = create_test_adapter();
        let password = "SuperSecretPassword123!";

        let hash_result = adapter.hash(password).await;
        assert!(hash_result.is_ok(), "密码哈希生成失败");

        let hash = hash_result.unwrap();
        assert!(!hash.is_empty());

        let verify_result = adapter.verify(password, &hash).await;
        assert!(verify_result.is_ok(), "正确密码应该通过验证");
    }

    #[tokio::test]
    async fn test_verify_wrong_password_fails() {
        let adapter = create_test_adapter();
        let password = "SuperSecretPassword123!";
        let wrong_password = "WrongPassword123!";

        let hash = adapter.hash(password).await.unwrap();

        let verify_result = adapter.verify(wrong_password, &hash).await;
        assert!(
            matches!(verify_result, Err(PasswordHasherError::Verify)),
            "错误密码应当返回 Verify 错误"
        );
    }

    #[tokio::test]
    async fn test_verify_malformed_hash_fails() {
        let adapter = create_test_adapter();
        let password = "SuperSecretPassword123!";
        let invalid_hash = "not_a_valid_phc_hash";

        let verify_result = adapter.verify(password, invalid_hash).await;
        assert!(
            matches!(verify_result, Err(PasswordHasherError::Verify)),
            "非法格式的 Hash 应当返回 Verify 错误"
        );
    }
}
