//! 全局中间件配置，来源于环境变量（前缀 `MIDDLEWARE_`）。
//! # 归属说明
//!
//! 放在 `platform-middleware` 内部而非 `apps/server`——遵循项目一贯
//! 原则"谁拥有实现，谁拥有配置"：`JwtConfig` 归 `platform-security`，
//! `DatabaseConfig` 归 `platform-database`，这里的 `MiddlewareConfig`
//! 描述的是本 crate 内部各中间件的行为参数，理应跟着实现走。
//! `apps/server` 只负责调用 `MiddlewareConfig::load()` 后传给 `apply()`，
//! 不持有配置定义本身。
//!
//! # 加载失败时的降级策略
//!
//! 与 `DatabaseConfig`/`JwtConfig` 等硬依赖不同——中间件配置错误不应
//! 阻止服务启动。数据库连不上，服务本来就没法正常工作；但限流阈值、
//! CORS 白名单这类配置解析失败，服务仍然可以用一套安全的默认值正常
//! 对外提供服务，因此调用方（`apps/server`）应在 `load()` 失败时落回
//! `MiddlewareConfig::default()` 并打一条 warn 日志，而不是让整个进程
//! 无法启动——这个降级逻辑放在调用方而非这里，因为"失败了该怎么办"
//! 是应用层的决策，不是配置定义本身的职责。

use platform_config::ConfigMeta;

use crate::{cors::CorsConfig, rate_limit::RateLimitConfig};

/// 全局中间件配置。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct MiddlewareConfig {
    /// 单个请求的最长处理时间（秒），超过后服务端主动中断。
    #[serde(default = "MiddlewareConfig::default_timeout_secs")]
    pub timeout_secs: u64,

    /// 请求体大小上限（字节），防止未认证的超大请求体耗尽内存。
    #[serde(default = "MiddlewareConfig::default_body_limit_bytes")]
    pub body_limit_bytes: usize,

    /// CORS 白名单等细节配置。
    #[serde(default)]
    pub cors: CorsConfig,

    /// 限流参数细节（阈值、窗口长度）。
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
}

impl MiddlewareConfig {
    const fn default_timeout_secs() -> u64 {
        30
    }

    const fn default_body_limit_bytes() -> usize {
        2 * 1024 * 1024 // 2 MiB
    }
}

impl ConfigMeta for MiddlewareConfig {
    type Error = std::convert::Infallible;

    fn prefix() -> &'static str {
        "MIDDLEWARE_"
    }

    fn validate(&self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl Default for MiddlewareConfig {
    fn default() -> Self {
        Self {
            timeout_secs: Self::default_timeout_secs(),
            body_limit_bytes: Self::default_body_limit_bytes(),
            cors: CorsConfig::default(),
            rate_limit: RateLimitConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_sane_thresholds() {
        let config = MiddlewareConfig::default();
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.body_limit_bytes, 2 * 1024 * 1024);
        assert_eq!(config.cors.allowed_origins, "");
        assert_eq!(config.rate_limit.max_requests, 100);
        assert_eq!(config.rate_limit.window_secs, 1);
    }

    #[test]
    fn load_from_reads_flat_and_nested_fields() {
        let vars = vec![
            ("MIDDLEWARE_TIMEOUT_SECS", "10"),
            ("MIDDLEWARE_BODY_LIMIT_BYTES", "1048576"),
            ("MIDDLEWARE_CORS__ALLOWED_ORIGINS", "http://x"),
            ("MIDDLEWARE_RATE_LIMIT__MAX_REQUESTS", "50"),
            ("MIDDLEWARE_RATE_LIMIT__WINDOW_SECS", "30"),
        ];
        let config = MiddlewareConfig::load_from(vars).unwrap();
        assert_eq!(config.timeout_secs, 10);
        assert_eq!(config.body_limit_bytes, 1048576);
        assert_eq!(config.cors.allowed_origins, "http://x");
        assert_eq!(config.rate_limit.max_requests, 50);
        assert_eq!(config.rate_limit.window_secs, 30);
    }

    /// 空输入 → 全部默认值
    #[test]
    fn load_from_applies_defaults_when_no_vars_present() {
        let config = MiddlewareConfig::load_from(Vec::<(String, String)>::new()).unwrap();
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.cors.allowed_origins, "");
        assert_eq!(config.rate_limit.max_requests, 100);
    }

    /// 变量名大小写不敏感
    #[test]
    fn env_keys_are_case_insensitive() {
        let config = MiddlewareConfig::load_from(vec![("middleware_timeout_secs", "5")]).unwrap();
        assert_eq!(config.timeout_secs, 5);
    }

    /// 非 MIDDLEWARE_ 前缀的键被忽略
    #[test]
    fn non_prefixed_keys_are_ignored() {
        let config = MiddlewareConfig::load_from(vec![("HTTP_TIMEOUT_SECS", "1")]).unwrap();
        assert_eq!(config.timeout_secs, 30);
    }

    #[test]
    fn prefix_is_middleware_and_validate_succeeds() {
        assert_eq!(MiddlewareConfig::prefix(), "MIDDLEWARE_");
        assert!(MiddlewareConfig::default().validate().is_ok());
    }
}
