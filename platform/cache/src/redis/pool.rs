//! 连接池的建立。

use deadpool_redis::{
    Config as DeadpoolConfig, Connection, Pool, PoolConfig, Runtime, Timeouts, redis::AsyncCommands,
};

use crate::{redis::config::RedisConfig, redis::error::RedisError};

/// Redis 连接池。
#[derive(Debug, Clone)]
pub struct RedisPool {
    pool: Pool,
}

impl RedisPool {
    /// 按配置建立连接池。
    pub fn connect(config: &RedisConfig) -> Result<Self, RedisError> {
        // 1. 显式校验配置参数，如果失败可直接抛出自己的自定义错误（或转化为相应的 RedisError）
        config
            .validate()
            .map_err(|e| RedisError::CommandFailed(e.to_string()))?;

        let mut deadpool_config = DeadpoolConfig::from_url(&config.url);

        // 2. 将超时配置设置在 `pool` 字段 (PoolConfig) 下
        deadpool_config.pool = Some(PoolConfig {
            timeouts: Timeouts {
                wait: Some(config.timeout()),
                ..Default::default()
            },
            ..Default::default()
        });

        let pool = deadpool_config
            .builder()
            .map_err(RedisError::ConfigInvalid)?
            .max_size(config.max_size)
            .runtime(Runtime::Tokio1)
            .build()
            .map_err(RedisError::BuildFailed)?;

        Ok(Self { pool })
    }

    /// 从池中取一个连接。
    pub async fn get_connection(&self) -> Result<Connection, RedisError> {
        self.pool.get().await.map_err(RedisError::AcquireFailed)
    }

    /// 健康检查：取一个连接并发一次 `PING`。
    pub async fn health_check(&self) -> Result<(), RedisError> {
        let mut conn = self.get_connection().await?;
        let _: String = conn
            .ping()
            .await
            .map_err(|e| RedisError::CommandFailed(e.to_string()))?;
        Ok(())
    }

    /// 获取底层连接池句柄。
    #[must_use]
    pub fn inner(&self) -> &Pool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_config() -> RedisConfig {
        RedisConfig {
            url: "redis://127.0.0.1:6379/0".to_string(),
            max_size: 4,
            timeout_secs: 1,
        }
    }

    fn config_with_url(url: &str) -> RedisConfig {
        RedisConfig {
            url: url.to_string(),
            ..valid_config()
        }
    }

    #[test]
    fn empty_url_fails_validation_before_connecting() {
        let config = config_with_url("   ");
        let result = RedisPool::connect(&config);
        assert!(matches!(result, Err(RedisError::CommandFailed(_))));
    }

    #[test]
    fn zero_max_size_fails_validation_before_connecting() {
        let mut config = valid_config();
        config.max_size = 0;
        let result = RedisPool::connect(&config);
        assert!(matches!(result, Err(RedisError::CommandFailed(_))));
    }

    #[test]
    fn malformed_url_returns_config_invalid_error() {
        let config = config_with_url("not-a-valid-redis-url");
        let result = RedisPool::connect(&config);
        assert!(matches!(result, Err(RedisError::ConfigInvalid(_))));
    }

    #[tokio::test]
    async fn acquiring_connection_to_unreachable_host_returns_acquire_failed_error() {
        // URL 格式合法，但目标地址不可达：错误发生在获取连接（真正尝试 TCP 连接）阶段，
        // 而不是 connect() 构造池本身——deadpool 的连接池是惰性的，构造阶段不会立即连接后端。
        let config = config_with_url("redis://127.0.0.1:1/0");
        let pool = RedisPool::connect(&config).unwrap();
        let result = pool.get_connection().await;
        assert!(matches!(result, Err(RedisError::AcquireFailed(_))));
    }

    #[tokio::test]
    async fn health_check_fails_on_unreachable_host() {
        let config = config_with_url("redis://127.0.0.1:1/0");
        let pool = RedisPool::connect(&config).unwrap();
        let result = pool.health_check().await;
        assert!(matches!(result, Err(RedisError::AcquireFailed(_))));
    }
}
