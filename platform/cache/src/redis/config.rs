//! 缓存连接配置。
//!
//! 对应环境变量前缀 `CACHE_`。

use std::time::Duration;

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

    /// 从环境变量加载（前缀 `CACHE_`）。
    pub fn load() -> Result<Self, platform_config::ConfigError> {
        platform_config::load_prefixed("CACHE_")
    }

    /// 启动阶段调用一次，失败即终止启动。
    pub fn validate(&self) -> Result<(), RedisConfigError> {
        if self.url.trim().is_empty() {
            return Err(RedisConfigError::EmptyUrl);
        }
        if self.max_size == 0 {
            return Err(RedisConfigError::ZeroMaxSize(self.max_size));
        }
        Ok(())
    }

    #[must_use]
    pub const fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs)
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

    #[test]
    fn defaults_applied_when_optional_env_vars_absent() {
        let vars = vec![("CACHE_URL".to_string(), "redis://localhost/0".to_string())];
        let cfg: RedisConfig = platform_config::load_prefixed_from("CACHE_", vars).unwrap();
        assert_eq!(cfg.max_size, 16);
        assert_eq!(cfg.timeout_secs, 5);
    }

    #[test]
    fn missing_required_url_fails_to_load() {
        let empty_vars = Vec::<(&str, &str)>::new();
        let result: Result<RedisConfig, _> =
            platform_config::load_prefixed_from("CACHE_", empty_vars);
        assert!(result.is_err());
    }

    #[test]
    fn load_uses_cache_prefix_consistently_with_load_prefixed() {
        let result = RedisConfig::load();
        if let Err(err) = result {
            assert!(err.to_string().contains("CACHE_"));
        }
    }
}
