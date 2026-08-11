//! 数据库连接配置。
//!
//! 对应环境变量前缀 `DATABASE_`。

use std::time::Duration;

use platform_config::ConfigMeta;
use serde::{Deserialize, Serialize};

/// 数据库连接池配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PgDatabaseConfig {
    /// 主库（读写）连接串。
    pub url: String,

    /// 只读副本连接串。缺省时读写都走 `url`（[`crate::pool::Pools::connect`]
    /// 内部会直接 `.clone()` 复用 write 连接池句柄，不建立第二个物理连接）。
    #[serde(default)]
    pub replica_url: Option<String>,

    #[serde(default = "PgDatabaseConfig::default_max_connections")]
    pub max_connections: u32,

    #[serde(default = "PgDatabaseConfig::default_min_connections")]
    pub min_connections: u32,

    /// 获取连接的超时时间（秒）：连接池已满且无空闲连接时，等待多久后放弃。
    #[serde(default = "PgDatabaseConfig::default_acquire_timeout_secs")]
    pub acquire_timeout_secs: u64,

    /// 单个连接的最大存活时间（秒），超过后连接池主动回收重建。
    #[serde(default = "PgDatabaseConfig::default_max_lifetime_secs")]
    pub max_lifetime_secs: u64,

    /// 连接最大空闲时间（秒），超过后被释放归还系统。
    #[serde(default = "PgDatabaseConfig::default_idle_timeout_secs")]
    pub idle_timeout_secs: u64,
}

impl Default for PgDatabaseConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            replica_url: None,
            max_connections: Self::default_max_connections(),
            min_connections: Self::default_min_connections(),
            acquire_timeout_secs: Self::default_acquire_timeout_secs(),
            max_lifetime_secs: Self::default_max_lifetime_secs(),
            idle_timeout_secs: Self::default_idle_timeout_secs(),
        }
    }
}

/// 配置语义层面的非法状态：字段本身能解析成功，但组合起来不合理。
#[derive(Debug, thiserror::Error)]
pub enum PgDatabaseConfigError {
    #[error("url 不能为空")]
    EmptyUrl,

    #[error("replica_url 不能为空字符串")]
    EmptyReplicaUrl,

    #[error("max_connections 必须大于 0，当前为 {0}")]
    ZeroMaxConnections(u32),

    #[error("min_connections({min}) 不应超过 max_connections({max})")]
    MinExceedsMax { min: u32, max: u32 },
}

impl PgDatabaseConfig {
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

impl ConfigMeta for PgDatabaseConfig {
    type Error = PgDatabaseConfigError;

    /// 统一使用 `DATABASE_` 环境变量前缀
    fn prefix() -> &'static str {
        "DATABASE_"
    }

    /// 启动阶段自我验证
    fn validate(&self) -> Result<(), Self::Error> {
        if self.url.trim().is_empty() {
            return Err(PgDatabaseConfigError::EmptyUrl);
        }
        if let Some(ref replica) = self.replica_url {
            if replica.trim().is_empty() {
                return Err(PgDatabaseConfigError::EmptyReplicaUrl);
            }
        }
        if self.max_connections == 0 {
            return Err(PgDatabaseConfigError::ZeroMaxConnections(
                self.max_connections,
            ));
        }
        if self.min_connections > self.max_connections {
            return Err(PgDatabaseConfigError::MinExceedsMax {
                min: self.min_connections,
                max: self.max_connections,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_config::ConfigError;

    fn valid_config() -> PgDatabaseConfig {
        PgDatabaseConfig {
            url: "postgres://user:pass@localhost/db".to_string(),
            replica_url: None,
            max_connections: 20,
            min_connections: 2,
            acquire_timeout_secs: 10,
            max_lifetime_secs: 1800,
            idle_timeout_secs: 600,
        }
    }

    // ---- validate 语义校验 ----

    #[test]
    fn valid_config_passes_validation() {
        assert!(valid_config().validate().is_ok());
    }

    #[test]
    fn empty_url_rejected() {
        let cfg = PgDatabaseConfig {
            url: "   ".to_string(),
            ..valid_config()
        };
        assert!(matches!(
            cfg.validate(),
            Err(PgDatabaseConfigError::EmptyUrl)
        ));
    }

    #[test]
    fn empty_replica_url_rejected() {
        let cfg = PgDatabaseConfig {
            replica_url: Some("   ".to_string()),
            ..valid_config()
        };
        assert!(matches!(
            cfg.validate(),
            Err(PgDatabaseConfigError::EmptyReplicaUrl)
        ));
    }

    #[test]
    fn zero_max_connections_rejected() {
        let cfg = PgDatabaseConfig {
            max_connections: 0,
            ..valid_config()
        };
        assert!(matches!(
            cfg.validate(),
            Err(PgDatabaseConfigError::ZeroMaxConnections(0))
        ));
    }

    #[test]
    fn min_exceeding_max_rejected() {
        let cfg = PgDatabaseConfig {
            min_connections: 30,
            max_connections: 20,
            ..valid_config()
        };
        assert!(matches!(
            cfg.validate(),
            Err(PgDatabaseConfigError::MinExceedsMax { min: 30, max: 20 })
        ));
    }

    // ---- has_replica ----

    #[test]
    fn has_replica_reflects_optional_field() {
        assert!(!valid_config().has_replica());
        let with_replica = PgDatabaseConfig {
            replica_url: Some("postgres://replica/db".to_string()),
            ..valid_config()
        };
        assert!(with_replica.has_replica());
    }

    #[test]
    fn has_replica_returns_false_for_blank_replica_url_even_without_prior_validation() {
        let cfg = PgDatabaseConfig {
            replica_url: Some("   ".to_string()),
            ..valid_config()
        };
        assert!(!cfg.has_replica());
    }

    #[test]
    fn has_replica_returns_false_for_empty_string_replica_url() {
        let cfg = PgDatabaseConfig {
            replica_url: Some(String::new()),
            ..valid_config()
        };
        assert!(!cfg.has_replica());
    }

    // ---- Duration 转换 ----

    #[test]
    fn acquire_timeout_converts_seconds_to_duration() {
        let cfg = PgDatabaseConfig {
            acquire_timeout_secs: 15,
            ..valid_config()
        };
        assert_eq!(cfg.acquire_timeout(), Duration::from_secs(15));
    }

    #[test]
    fn max_lifetime_converts_seconds_to_duration() {
        let cfg = PgDatabaseConfig {
            max_lifetime_secs: 900,
            ..valid_config()
        };
        assert_eq!(cfg.max_lifetime(), Duration::from_secs(900));
    }

    #[test]
    fn idle_timeout_converts_seconds_to_duration() {
        let cfg = PgDatabaseConfig {
            idle_timeout_secs: 300,
            ..valid_config()
        };
        assert_eq!(cfg.idle_timeout(), Duration::from_secs(300));
    }

    // ---- 加载测试：ConfigMeta::load_from 返回 Result，与生产 load() 共用同一实现 ----
    // 快乐路径 unwrap；错误路径用 matches! 区分 Load（反序列化失败）与
    // Validation（反序列化成功但语义非法）。

    #[test]
    fn defaults_applied_when_optional_env_vars_absent() {
        let cfg = PgDatabaseConfig::load_from(vec![("DATABASE_URL", "postgres://x/y")]).unwrap();
        assert_eq!(cfg.max_connections, 20);
        assert_eq!(cfg.min_connections, 2);
        assert_eq!(cfg.idle_timeout_secs, 600);
        assert!(cfg.replica_url.is_none());
    }

    #[test]
    fn replica_url_loaded_when_present() {
        let cfg = PgDatabaseConfig::load_from(vec![
            ("DATABASE_URL", "postgres://primary/db"),
            ("DATABASE_REPLICA_URL", "postgres://replica/db"),
        ])
        .unwrap();
        assert_eq!(cfg.replica_url.as_deref(), Some("postgres://replica/db"));
    }

    /// 必填字段缺失（无 DATABASE_URL）→ Load 错误（figment 报 missing field）
    #[test]
    fn missing_required_url_is_load_error() {
        let result = PgDatabaseConfig::load_from(Vec::<(&str, &str)>::new());
        assert!(matches!(result, Err(ConfigError::Load(_))));
    }

    /// 反序列化成功但语义非法（url 为空串）→ Validation 错误
    #[test]
    fn empty_url_via_load_from_is_validation_error() {
        let result = PgDatabaseConfig::load_from(vec![("DATABASE_URL", "")]);
        assert!(matches!(result, Err(ConfigError::Validation { .. })));
    }

    /// 反序列化成功但语义非法（max_connections=0）→ Validation 错误
    #[test]
    fn zero_max_connections_via_load_from_is_validation_error() {
        let result = PgDatabaseConfig::load_from(vec![
            ("DATABASE_URL", "postgres://x/y"),
            ("DATABASE_MAX_CONNECTIONS", "0"),
        ]);
        assert!(matches!(result, Err(ConfigError::Validation { .. })));
    }

    /// 反序列化成功但语义非法（min > max）→ Validation 错误
    #[test]
    fn min_exceeding_max_via_load_from_is_validation_error() {
        let result = PgDatabaseConfig::load_from(vec![
            ("DATABASE_URL", "postgres://x/y"),
            ("DATABASE_MIN_CONNECTIONS", "30"),
            ("DATABASE_MAX_CONNECTIONS", "20"),
        ]);
        assert!(matches!(result, Err(ConfigError::Validation { .. })));
    }

    /// 值无法解析成目标类型 → Load 错误
    #[test]
    fn non_numeric_max_connections_is_load_error() {
        let result = PgDatabaseConfig::load_from(vec![
            ("DATABASE_URL", "postgres://x/y"),
            ("DATABASE_MAX_CONNECTIONS", "many"),
        ]);
        assert!(matches!(result, Err(ConfigError::Load(_))));
    }

    /// load() 是生产方法，从真实环境变量读取，仍返回 Result
    #[test]
    fn load_uses_database_prefix_consistently() {
        let result = PgDatabaseConfig::load();
        if let Err(err) = result {
            // 缺少 DATABASE_URL 时，加载错误信息应包含相关特征
            let err_msg = err.to_string();
            assert!(
                err_msg.contains("DATABASE_") || err_msg.contains("url"),
                "实际错误信息为: {err_msg}"
            );
        }
    }
}
