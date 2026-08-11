//! 缓存连接配置。
//!
//! 对应环境变量前缀 `REDIS_`。

use std::time::Duration;

use platform_config::ConfigMeta;
use serde::{Deserialize, Serialize};

/// Redis 连接池配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis 连接串，如 `redis://user:pass@host:6379/0`。
    pub url: String,

    #[serde(default = "RedisConfig::default_max_size")]
    pub max_size: usize,

    /// 获取连接的超时时间（秒）：连接池已满时等待多久后放弃。
    #[serde(default = "RedisConfig::default_timeout_secs")]
    pub timeout_secs: u64,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            max_size: Self::default_max_size(),
            timeout_secs: Self::default_timeout_secs(),
        }
    }
}

/// 配置语义层面的非法状态。
#[derive(Debug, thiserror::Error)]
pub enum RedisConfigError {
    #[error("url 不能为空")]
    EmptyUrl,

    #[error("max_size 必须大于 0，当前为 {0}")]
    ZeroMaxSize(usize),
}

impl RedisConfig {
    const fn default_max_size() -> usize {
        16
    }

    const fn default_timeout_secs() -> u64 {
        5
    }

    #[must_use]
    pub const fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs)
    }
}

impl ConfigMeta for RedisConfig {
    type Error = RedisConfigError;

    /// 统一使用 `REDIS_` 环境变量前缀
    fn prefix() -> &'static str {
        "REDIS_"
    }

    /// 启动阶段自我验证
    fn validate(&self) -> Result<(), Self::Error> {
        if self.url.trim().is_empty() {
            return Err(RedisConfigError::EmptyUrl);
        }
        if self.max_size == 0 {
            return Err(RedisConfigError::ZeroMaxSize(self.max_size));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_config::ConfigError;

    fn valid_config() -> RedisConfig {
        RedisConfig {
            url: "redis://localhost:6379/0".to_string(),
            max_size: 16,
            timeout_secs: 5,
        }
    }

    // ---- validate() 语义校验 ----

    #[test]
    fn valid_config_passes_validation() {
        assert!(valid_config().validate().is_ok());
    }

    #[test]
    fn empty_url_rejected() {
        let cfg = RedisConfig {
            url: "   ".to_string(),
            ..valid_config()
        };
        assert!(matches!(cfg.validate(), Err(RedisConfigError::EmptyUrl)));
    }

    #[test]
    fn zero_max_size_rejected() {
        let cfg = RedisConfig {
            max_size: 0,
            ..valid_config()
        };
        assert!(matches!(
            cfg.validate(),
            Err(RedisConfigError::ZeroMaxSize(0))
        ));
    }

    #[test]
    fn timeout_converts_seconds_to_duration() {
        let cfg = RedisConfig {
            timeout_secs: 10,
            ..valid_config()
        };
        assert_eq!(cfg.timeout(), Duration::from_secs(10));
    }

    // ---- 加载测试：load_from 返回 Result，与生产 load() 共用同一实现 ----
    // 快乐路径 unwrap；错误路径用 matches! 区分 Load（反序列化失败）与
    // Validation（反序列化成功但语义非法）。

    #[test]
    fn defaults_applied_when_optional_env_vars_absent() {
        let vars = vec![("REDIS_URL", "redis://localhost/0")];
        let cfg = RedisConfig::load_from(vars).unwrap();
        assert_eq!(cfg.max_size, 16);
        assert_eq!(cfg.timeout_secs, 5);
    }

    #[test]
    fn all_fields_loaded_via_load_from() {
        let vars = vec![
            ("REDIS_URL", "redis://user:pass@localhost:6379/2"),
            ("REDIS_MAX_SIZE", "32"),
            ("REDIS_TIMEOUT_SECS", "10"),
        ];
        let cfg = RedisConfig::load_from(vars).unwrap();
        assert_eq!(cfg.url, "redis://user:pass@localhost:6379/2");
        assert_eq!(cfg.max_size, 32);
        assert_eq!(cfg.timeout_secs, 10);
    }

    /// 变量名大小写不敏感（前缀匹配与小写归一化后等价）
    #[test]
    fn env_keys_are_case_insensitive() {
        let cfg = RedisConfig::load_from(vec![
            ("redis_url", "redis://localhost/0"),
            ("REDIS_MAX_SIZE", "8"),
        ])
        .unwrap();
        assert_eq!(cfg.url, "redis://localhost/0");
        assert_eq!(cfg.max_size, 8);
    }

    /// 非 REDIS_ 前缀的键被忽略，不影响默认值
    #[test]
    fn non_prefixed_keys_are_ignored() {
        let cfg = RedisConfig::load_from(vec![
            ("REDIS_URL", "redis://localhost/0"),
            ("CACHE_MAX_SIZE", "9999"), // 非 REDIS_ 前缀 → 忽略
        ])
        .unwrap();
        assert_eq!(cfg.max_size, 16); // 默认值未被污染
    }

    /// 必填字段缺失（无 REDIS_URL）→ Load 错误（figment 报 missing field）
    #[test]
    fn missing_required_url_is_load_error() {
        let result = RedisConfig::load_from(Vec::<(&str, &str)>::new());
        assert!(matches!(result, Err(ConfigError::Load(_))));
    }

    /// 反序列化成功但语义非法（url 为空串）→ Validation 错误
    #[test]
    fn empty_url_via_load_from_is_validation_error() {
        let result = RedisConfig::load_from(vec![("REDIS_URL", "")]);
        assert!(matches!(result, Err(ConfigError::Validation { .. })));
    }

    /// 反序列化成功但语义非法（max_size=0）→ Validation 错误
    #[test]
    fn zero_max_size_via_load_from_is_validation_error() {
        let result = RedisConfig::load_from(vec![
            ("REDIS_URL", "redis://localhost:6379/0"),
            ("REDIS_MAX_SIZE", "0"),
        ]);
        assert!(matches!(result, Err(ConfigError::Validation { .. })));
    }

    /// 值无法解析成目标类型 → Load 错误
    #[test]
    fn non_numeric_max_size_is_load_error() {
        let result = RedisConfig::load_from(vec![
            ("REDIS_URL", "redis://localhost:6379/0"),
            ("REDIS_MAX_SIZE", "many"),
        ]);
        assert!(matches!(result, Err(ConfigError::Load(_))));
    }

    /// load() 是生产方法，从真实环境变量读取，仍返回 Result
    #[test]
    fn load_uses_redis_prefix_consistently() {
        let result = RedisConfig::load();
        if let Err(err) = result {
            let err_msg = err.to_string();
            assert!(
                err_msg.contains("REDIS_") || err_msg.contains("url"),
                "实际错误信息为: {err_msg}"
            );
        }
    }
}
