//! 连接池的建立。

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::pg::config::PgDatabaseConfig;
use crate::pg::error::PgDatabaseError;

/// 读写连接池。
///
/// `write`：命令路径（insert/update/delete）使用。
/// `read`：查询路径使用。未配置 replica 时与 `write` 指向同一连接池
/// （见模块文档），代码写法与真正分离的场景完全一致。
///
/// `Clone`：内部字段都是 `sqlx::PgPool`，其本身就是 `Arc` 包装，
/// `.clone()` 是廉价的句柄复制，可以放心地把 `PgPools` 按值传给多个
/// repository 持有，不需要额外包一层 `Arc<PgPools>`（包不包全看调用方
/// 是否还需要共享其他非 Clone 的字段，`PgPools` 自身已经足够廉价可 clone）。
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

    /// 健康检查：向 write pool 发一次最简单的查询，确认连接确实可用。
    /// 增加 2 秒硬超时，防止数据库悬挂拖垮 `/healthz` 端点。
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

    /// 辅助函数：构造一个通过了 validate 的合法基础配置。
    /// `ConfigMeta::load_from` 返回 `Result`，此处输入恒合法，直接 unwrap。
    fn valid_test_config(url: &str) -> PgDatabaseConfig {
        PgDatabaseConfig::load_from(vec![
            ("DATABASE_URL", url),
            ("DATABASE_MAX_CONNECTIONS", "5"),
            ("DATABASE_MIN_CONNECTIONS", "0"),
            ("DATABASE_ACQUIRE_TIMEOUT_SECS", "2"),
        ])
        .unwrap()
    }

    // pool 层的单测只关注网络/连接失败的映射，不关注配置字段非法
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
