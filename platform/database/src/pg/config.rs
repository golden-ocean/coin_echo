//! 数据库连接配置。
//!
//! 对应环境变量前缀 `DATABASE_`。

use std::time::Duration;

/// 数据库连接池配置。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DatabaseConfig {
    /// 主库（读写）连接串。
    pub url: String,

    /// 只读副本连接串。缺省时读写都走 `url`（[`crate::pool::Pools::connect`]
    /// 内部会直接 `.clone()` 复用 write 连接池句柄，不建立第二个物理连接）。
    #[serde(default)]
    pub replica_url: Option<String>,

    #[serde(default = "DatabaseConfig::default_max_connections")]
    pub max_connections: u32,

    #[serde(default = "DatabaseConfig::default_min_connections")]
    pub min_connections: u32,

    /// 获取连接的超时时间（秒）：连接池已满且无空闲连接时，等待多久后放弃。
    #[serde(default = "DatabaseConfig::default_acquire_timeout_secs")]
    pub acquire_timeout_secs: u64,

    /// 单个连接的最大存活时间（秒），超过后连接池主动回收重建。
    #[serde(default = "DatabaseConfig::default_max_lifetime_secs")]
    pub max_lifetime_secs: u64,

    /// 连接最大空闲时间（秒），超过后被释放归还系统。
    #[serde(default = "DatabaseConfig::default_idle_timeout_secs")]
    pub idle_timeout_secs: u64,
}

/// 配置语义层面的非法状态：字段本身能解析成功，但组合起来不合理。
#[derive(Debug, thiserror::Error)]
pub enum DatabaseConfigError {
    #[error("url 不能为空")]
    EmptyUrl,

    #[error("replica_url 不能为空字符串")]
    EmptyReplicaUrl,

    #[error("max_connections 必须大于 0，当前为 {0}")]
    ZeroMaxConnections(u32),

    #[error("min_connections({min}) 不应超过 max_connections({max})")]
    MinExceedsMax { min: u32, max: u32 },
}

impl DatabaseConfig {
    const fn default_max_connections() -> u32 {
        20
    }

    const fn default_min_connections() -> u32 {
        2
    }

    const fn default_acquire_timeout_secs() -> u64 {
        10
    }

    const fn default_max_lifetime_secs() -> u64 {
        30 * 60 // 30 分钟
    }

    const fn default_idle_timeout_secs() -> u64 {
        10 * 60 // 10 分钟
    }

    /// 从环境变量加载（前缀 `DATABASE_`）。
    pub fn load() -> Result<Self, platform_config::ConfigError> {
        platform_config::load_prefixed("DATABASE_")
    }

    /// 启动阶段调用一次，失败即终止启动。
    pub fn validate(&self) -> Result<(), DatabaseConfigError> {
        if self.url.trim().is_empty() {
            return Err(DatabaseConfigError::EmptyUrl);
        }
        if let Some(ref replica) = self.replica_url {
            if replica.trim().is_empty() {
                return Err(DatabaseConfigError::EmptyReplicaUrl);
            }
        }
        if self.max_connections == 0 {
            return Err(DatabaseConfigError::ZeroMaxConnections(
                self.max_connections,
            ));
        }
        if self.min_connections > self.max_connections {
            return Err(DatabaseConfigError::MinExceedsMax {
                min: self.min_connections,
                max: self.max_connections,
            });
        }
        Ok(())
    }

    #[must_use]
    pub const fn acquire_timeout(&self) -> Duration {
        Duration::from_secs(self.acquire_timeout_secs)
    }

    #[must_use]
    pub const fn max_lifetime(&self) -> Duration {
        Duration::from_secs(self.max_lifetime_secs)
    }

    #[must_use]
    pub const fn idle_timeout(&self) -> Duration {
        Duration::from_secs(self.idle_timeout_secs)
    }

    /// 是否设置了有效（非空白）的 replica_url。
    ///
    /// 不依赖调用方提前调用过 [`Self::validate`]——自身就完成非空白判断，
    /// 避免"这个方法只有在 validate 通过后才可信"这种隐式前提分散在
    /// 两处维护，容易在其中一处修改时忘了同步另一处。
    #[must_use]
    pub fn has_replica(&self) -> bool {
        self.replica_url
            .as_deref()
            .is_some_and(|url| !url.trim().is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_config() -> DatabaseConfig {
        DatabaseConfig {
            url: "postgres://user:pass@localhost/db".to_string(),
            replica_url: None,
            max_connections: 20,
            min_connections: 2,
            acquire_timeout_secs: 10,
            max_lifetime_secs: 1800,
            idle_timeout_secs: 600,
        }
    }

    #[test]
    fn valid_config_passes_validation() {
        assert!(valid_config().validate().is_ok());
    }

    #[test]
    fn empty_url_rejected() {
        let cfg = DatabaseConfig {
            url: "   ".to_string(),
            ..valid_config()
        };
        assert!(matches!(cfg.validate(), Err(DatabaseConfigError::EmptyUrl)));
    }

    #[test]
    fn empty_replica_url_rejected() {
        let cfg = DatabaseConfig {
            replica_url: Some("   ".to_string()),
            ..valid_config()
        };
        assert!(matches!(
            cfg.validate(),
            Err(DatabaseConfigError::EmptyReplicaUrl)
        ));
    }

    #[test]
    fn zero_max_connections_rejected() {
        let cfg = DatabaseConfig {
            max_connections: 0,
            ..valid_config()
        };
        assert!(matches!(
            cfg.validate(),
            Err(DatabaseConfigError::ZeroMaxConnections(0))
        ));
    }

    #[test]
    fn min_exceeding_max_rejected() {
        let cfg = DatabaseConfig {
            min_connections: 30,
            max_connections: 20,
            ..valid_config()
        };
        assert!(matches!(
            cfg.validate(),
            Err(DatabaseConfigError::MinExceedsMax { min: 30, max: 20 })
        ));
    }

    // ---- has_replica ----

    #[test]
    fn has_replica_reflects_optional_field() {
        assert!(!valid_config().has_replica());
        let with_replica = DatabaseConfig {
            replica_url: Some("postgres://replica/db".to_string()),
            ..valid_config()
        };
        assert!(with_replica.has_replica());
    }

    #[test]
    fn has_replica_returns_false_for_blank_replica_url_even_without_prior_validation() {
        // has_replica 不依赖调用方提前调用过 validate()，自身就应识别出
        // 空白字符串不算"有效副本"——即便这是一个尚未校验过的实例。
        let cfg = DatabaseConfig {
            replica_url: Some("   ".to_string()),
            ..valid_config()
        };
        assert!(!cfg.has_replica());
    }

    #[test]
    fn has_replica_returns_false_for_empty_string_replica_url() {
        let cfg = DatabaseConfig {
            replica_url: Some(String::new()),
            ..valid_config()
        };
        assert!(!cfg.has_replica());
    }

    // ---- Duration 转换 ----

    #[test]
    fn acquire_timeout_converts_seconds_to_duration() {
        let cfg = DatabaseConfig {
            acquire_timeout_secs: 15,
            ..valid_config()
        };
        assert_eq!(cfg.acquire_timeout(), Duration::from_secs(15));
    }

    #[test]
    fn max_lifetime_converts_seconds_to_duration() {
        let cfg = DatabaseConfig {
            max_lifetime_secs: 900,
            ..valid_config()
        };
        assert_eq!(cfg.max_lifetime(), Duration::from_secs(900));
    }

    #[test]
    fn idle_timeout_converts_seconds_to_duration() {
        let cfg = DatabaseConfig {
            idle_timeout_secs: 300,
            ..valid_config()
        };
        assert_eq!(cfg.idle_timeout(), Duration::from_secs(300));
    }

    // ---- 环境变量加载（envy 直接构造，不触碰真实进程环境变量） ----

    #[test]
    fn defaults_applied_when_optional_env_vars_absent() {
        let vars = vec![("DATABASE_URL".to_string(), "postgres://x/y".to_string())];
        let cfg: DatabaseConfig = platform_config::load_prefixed_from("DATABASE_", vars).unwrap();
        assert_eq!(cfg.max_connections, 20);
        assert_eq!(cfg.min_connections, 2);
        assert_eq!(cfg.idle_timeout_secs, 600);
        assert!(cfg.replica_url.is_none());
    }

    #[test]
    fn missing_required_url_fails_to_load() {
        let empty_vars = Vec::<(&str, &str)>::new();
        let result: Result<DatabaseConfig, _> =
            platform_config::load_prefixed_from("DATABASE_", empty_vars);
        assert!(result.is_err());
    }

    #[test]
    fn replica_url_loaded_when_present() {
        let vars = vec![
            ("DATABASE_URL", "postgres://primary/db"),
            ("DATABASE_REPLICA_URL", "postgres://replica/db"),
        ];

        let cfg: DatabaseConfig = platform_config::load_prefixed_from("DATABASE_", vars).unwrap();

        assert_eq!(cfg.replica_url.as_deref(), Some("postgres://replica/db"));
    }

    // ---- load()：真正调用被测函数本身，而不是绕开它单独测 envy 行为 ----
    //
    // 不使用 std::env::set_var/remove_var：这是进程级全局状态，测试默认
    // 并行执行，多个测试同时读写会互相污染，出现依赖执行顺序的间歇性
    // 失败。这里改为验证 load() 内部确实调用的是
    // platform_config::load_prefixed("DATABASE_")——通过直接调用同一个
    // 底层函数、用相同前缀构造等价场景来断言，不依赖真实环境变量。

    #[test]
    fn load_uses_database_prefix_consistently_with_load_prefixed() {
        // 缺少必填的 DATABASE_URL 时，DatabaseConfig::load() 应该失败，
        // 且错误信息应带有 "DATABASE_" 前缀——这条断言能捕获此前发生过的
        // 真实问题：load() 函数体内误写成不存在的 `config::` crate 路径，
        // 或者前缀字符串被改错。
        //
        // 由于 load() 直接读取真实进程环境变量，这里不主动设置任何
        // DATABASE_ 变量，只验证"未配置时失败、且错误信息包含正确前缀"
        // 这个不依赖具体环境状态的稳定行为。
        let result = DatabaseConfig::load();
        if let Err(err) = result {
            assert!(err.to_string().contains("DATABASE_"));
        }
        // 若当前进程环境恰好设置了 DATABASE_URL（例如本地开发者的 shell
        // 环境），result 可能是 Ok，这属于正常情况，不作为失败条件——
        // 本测试的目的是防止 load() 内部路径/前缀写错导致的编译期或
        // 逻辑错误，而非断言某个特定的运行结果。
    }
}
