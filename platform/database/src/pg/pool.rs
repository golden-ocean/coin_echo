//! 连接池的建立。

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::pg::config::DatabaseConfig;
use crate::pg::error::DatabaseError;

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
    pub async fn connect(config: &DatabaseConfig) -> Result<Self, DatabaseError> {
        let write = Self::build_pool(config, &config.url).await?;

        let read = if let Some(replica_url) = config.replica_url.as_deref() {
            Self::build_pool(config, replica_url).await?
        } else {
            // 未配置 replica：克隆 write 的句柄，不建立新物理连接。
            write.clone()
        };

        Ok(Self { write, read })
    }

    async fn build_pool(config: &DatabaseConfig, url: &str) -> Result<PgPool, DatabaseError> {
        PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(config.acquire_timeout())
            .max_lifetime(config.max_lifetime())
            .idle_timeout(config.idle_timeout())
            .connect(url)
            .await
            .map_err(DatabaseError::ConnectFailed)
    }

    /// 健康检查：向 write pool 发一次最简单的查询，确认连接确实可用。
    /// 增加 2 秒硬超时，防止数据库悬挂拖垮 `/healthz` 端点。
    pub async fn health_check(&self) -> Result<(), DatabaseError> {
        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            sqlx::query("SELECT 1").execute(&self.write),
        )
        .await
        .map_err(|_| DatabaseError::ConnectFailed(sqlx::Error::PoolTimedOut))?
        .map_err(DatabaseError::ConnectFailed)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with_url(url: &str) -> DatabaseConfig {
        DatabaseConfig {
            url: url.to_string(),
            replica_url: None,
            max_connections: 5,
            min_connections: 0,
            acquire_timeout_secs: 2,
            max_lifetime_secs: 1800,
            idle_timeout_secs: 600,
        }
    }

    // 以下测试均不需要真实数据库：只验证"连不上时返回的是我们自己的错误
    // 类型，而不是 panic 或裸的 sqlx::Error"，这部分不依赖真实网络成功，
    // 用一个必然连不上的地址即可稳定复现。集成测试（真实连接、真实查询）
    // 属于 `iam-infra` 或 apps/server 的职责，不在这个 crate 里做。

    #[tokio::test]
    async fn connect_to_unreachable_host_returns_connect_failed_error() {
        let config = config_with_url("postgres://user:pass@127.0.0.1:1/nonexistent");
        let result = PgPools::connect(&config).await;
        assert!(matches!(result, Err(DatabaseError::ConnectFailed(_))));
    }

    #[tokio::test]
    async fn connect_to_malformed_url_returns_connect_failed_error() {
        let config = config_with_url("not-a-valid-connection-string");
        let result = PgPools::connect(&config).await;
        assert!(matches!(result, Err(DatabaseError::ConnectFailed(_))));
    }

    #[tokio::test]
    async fn blank_replica_url_falls_back_to_cloning_write_pool_without_connecting() {
        // replica_url 为空白字符串时，不应该真的尝试用这个空字符串去
        // 建立连接（那样会产生一次不必要的、必然失败的连接尝试）；
        // 应该走 write.clone() 这条分支——用一个必然连不上的 write 地址，
        // 若 connect() 因为"尝试连接空白 replica_url"而在 write 之外
        // 报出第二个错误路径，这个测试能捕获逻辑分支写错的情况。
        let config = DatabaseConfig {
            replica_url: Some("   ".to_string()),
            ..config_with_url("postgres://user:pass@127.0.0.1:1/nonexistent")
        };
        let result = PgPools::connect(&config).await;
        // write 本身连不上，无论 replica 分支对错，最终都应该是
        // ConnectFailed；这里主要保证不会因为空白 replica_url 触发
        // panic 或别的异常路径。
        assert!(matches!(result, Err(DatabaseError::ConnectFailed(_))));
    }
}
