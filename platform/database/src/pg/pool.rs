//! 连接池的建立与管理。

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::pg::config::PgDatabaseConfig;
use crate::pg::error::PgDatabaseError;

/// 读写连接池。
///
/// `write`：写路径（INSERT/UPDATE/DELETE）使用。
/// `read`：读路径使用。未配置 replica 时与 `write` 指向同一连接池，
/// `.clone()` 内部仅复制 Arc 句柄，避免创建重复的物理连接。
#[derive(Debug, Clone)]
pub struct PgPools {
    pub write: PgPool,
    pub read: PgPool,
}

impl PgPools {
    /// 按配置建立连接池。
    pub async fn connect(cfg: &PgDatabaseConfig) -> Result<Self, PgDatabaseError> {
        let write = Self::build_pool(cfg, &cfg.url).await?;

        let read = if cfg.has_replica() {
            let replica_url = cfg.replica_url.as_deref().unwrap().trim();
            Self::build_pool(cfg, replica_url).await?
        } else {
            // 未配置或配置为空白 replica：克隆 write 句柄，不建立新物理连接
            write.clone()
        };

        Ok(Self { write, read })
    }

    async fn build_pool(config: &PgDatabaseConfig, url: &str) -> Result<PgPool, PgDatabaseError> {
        PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(config.acquire_timeout())
            .max_lifetime(config.max_lifetime())
            .idle_timeout(config.idle_timeout())
            .connect(url)
            .await
            .map_err(PgDatabaseError::ConnectFailed)
    }

    /// 健康检查：向 write pool 发送简单查询，确认数据库连通性。
    /// 增加 2 秒硬超时，防止数据库无响应阻塞服务健康检查接口。
    pub async fn health_check(&self) -> Result<(), PgDatabaseError> {
        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            sqlx::query("SELECT 1").execute(&self.write),
        )
        .await
        .map_err(|_| PgDatabaseError::ConnectFailed(sqlx::Error::PoolTimedOut))?
        .map_err(PgDatabaseError::ConnectFailed)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_config::ConfigMeta;

    /// 辅助函数：构造一个符合校验规则的基础配置。
    fn valid_test_config(url: &str) -> PgDatabaseConfig {
        PgDatabaseConfig::load_from(vec![
            ("DATABASE_URL", url),
            ("DATABASE_MAX_CONNECTIONS", "5"),
            ("DATABASE_MIN_CONNECTIONS", "0"),
            ("DATABASE_ACQUIRE_TIMEOUT_SECS", "2"),
        ])
        .unwrap()
    }

    #[tokio::test]
    async fn connect_to_unreachable_host_returns_connect_failed_error() {
        let config = valid_test_config("postgres://user:pass@127.0.0.1:1/nonexistent");
        let result = PgPools::connect(&config).await;
        assert!(matches!(result, Err(PgDatabaseError::ConnectFailed(_))));
    }

    #[tokio::test]
    async fn connect_to_malformed_url_returns_connect_failed_error() {
        let config = valid_test_config("not-a-valid-connection-string");
        let result = PgPools::connect(&config).await;
        assert!(matches!(result, Err(PgDatabaseError::ConnectFailed(_))));
    }
}
