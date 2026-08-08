//! 缓存相关错误。

use platform_kernel::error::{ErrorKind, ErrorMeta};

/// 缓存连接过程中的错误。
///
/// 只覆盖"连接池建不起来"这类启动阶段的故障。具体缓存操作（get/set 等）
/// 失败时如何分类、是否需要 fallback 到数据库，属于调用方（业务 crate）
/// 的职责——例如"缓存未命中"不是错误而是正常路径的一种，不应该被这里的
/// 类型代为决定。
/// 缓存连接过程中的错误。
#[derive(Debug, thiserror::Error)]
pub enum RedisError {
    /// 连接串配置非法（`.builder()` 阶段）。
    #[error("缓存连接配置无效：{0}")]
    ConfigInvalid(#[source] deadpool_redis::ConfigError),

    /// 构建连接池失败（`.build()` 阶段，如 max_size 为 0 等参数非法）。
    #[error("构建缓存连接池失败：{0}")]
    BuildFailed(#[source] deadpool_redis::BuildError),

    /// 从连接池获取连接失败：池已耗尽、后端不可达、超时等。
    #[error("获取缓存连接失败：{0}")]
    AcquireFailed(#[source] deadpool_redis::PoolError),

    #[error("执行命令失败: {0}")]
    CommandFailed(String),
}

impl ErrorMeta for RedisError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Unavailable
    }

    fn code(&self) -> &'static str {
        match self {
            Self::ConfigInvalid(_) => "redis.config_invalid",
            Self::BuildFailed(_) => "redis.build_failed",
            Self::AcquireFailed(_) => "redis.acquire_failed",
            Self::CommandFailed(_) => "redis.command_failed",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_failure_is_unavailable_and_retryable() {
        let err = RedisError::AcquireFailed(deadpool_redis::PoolError::Closed);
        assert_eq!(err.kind(), ErrorKind::Unavailable);
        assert!(err.retryable());
    }

    #[test]
    fn codes_are_distinct() {
        let acquire_err = RedisError::AcquireFailed(deadpool_redis::PoolError::Closed);
        assert_eq!(acquire_err.code(), "redis.acquire_failed");
        let command_err = RedisError::CommandFailed("command failed".to_string());
        assert_eq!(command_err.code(), "redis.command_failed");
    }
}
