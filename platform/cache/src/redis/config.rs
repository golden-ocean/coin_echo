//! 缓存连接配置。
//!
//! 对应环境变量前缀 `REDIS_`。

use std::time::Duration;

use platform_config::ConfigMeta;
use serde::Deserialize;

/// Redis 连接池配置。
#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    /// Redis 连接串，如 `redis://user:pass@host:6379/0`。
    pub url: String,

    #[serde(default = "RedisConfig::default_max_size")]
    pub max_size: usize,

    /// 获取连接的超时时间（秒）：连接池已满时等待多久后放弃。
    #[serde(default = "RedisConfig::default_timeout_secs")]
    pub timeout_secs: u64,
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

    fn valid_config() -> RedisConfig {
        RedisConfig {
            url: "redis://localhost:6379/0".to_string(),
            max_size: 16,
            timeout_secs: 5,
        }
    }

    #[test]
    fn valid_config_passes_validation() {
        assert!(valid_config().validate().is_ok());
    }

    #[test]
    fn empty_url_rejected() {
        let cfg = RedisConfig {
            url: "  ".to_string(),
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

    // ---- 基于 ConfigMeta::load_from 的测试 ----

    #[test]
    fn defaults_applied_when_optional_env_vars_absent() {
        let vars = vec![("REDIS_URL", "redis://localhost/0")];
        // load_from 会自动应用前缀、反序列化默认值，并触发 validate()
        let cfg = RedisConfig::load_from(vars).unwrap();
        assert_eq!(cfg.max_size, 16);
        assert_eq!(cfg.timeout_secs, 5);
    }

    #[test]
    fn missing_required_url_fails_to_load() {
        let empty_vars = Vec::<(&str, &str)>::new();
        let result = RedisConfig::load_from(empty_vars);
        assert!(result.is_err());
    }

    #[test]
    fn load_from_validates_semantic_rules() {
        // 测试经过 load_from 反序列化成功，但未通过 validate() 业务校验的情况
        let invalid_vars = vec![
            ("REDIS_URL", "redis://localhost/0"),
            ("REDIS_MAX_SIZE", "0"), // 触发 ZeroMaxSize 错误
        ];
        let result = RedisConfig::load_from(invalid_vars);
        assert!(result.is_err());

        // 确认错误信息中包含了正确的 REDIS_ 前缀
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("REDIS_"));
    }
}
