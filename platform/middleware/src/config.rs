//! 全局中间件配置，逐项可开关，来源于环境变量（前缀 `MIDDLEWARE_`）。
//!
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
    /// 是否生成/传播 `x-request-id` 请求头，以及安装
    /// [`crate::context::RequestContextLayer`]（两者共用一个开关：
    /// context 依赖 request_id 已经写入请求头，关掉 request_id 时
    /// context 也没有意义继续安装）。

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

    /// 目前没有跨字段的语义约束需要校验（各布尔开关互相独立，
    /// timeout/body_limit 是纯数值，取值范围由业务判断，不在这里
    /// 强制限定）。使用 `Infallible` 表示"这个校验永远不会失败"，
    /// 比返回一个没有任何变体的空枚举更符合 Rust 惯例。
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
    fn default_enables_everything_with_sane_thresholds() {
        let config = MiddlewareConfig::default();
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.body_limit_bytes, 2 * 1024 * 1024);
    }

    #[test]
    fn load_from_applies_defaults_when_no_vars_present() {
        let config = MiddlewareConfig::load_from(Vec::<(String, String)>::new()).unwrap();
        assert_eq!(config.timeout_secs, 30);
    }

    #[test]
    fn load_from_respects_individual_toggle_overrides() {
        let vars = vec![
            ("MIDDLEWARE_TRACE_ENABLED".to_string(), "false".to_string()),
            ("MIDDLEWARE_TIMEOUT_SECS".to_string(), "5".to_string()),
        ];
        let config = MiddlewareConfig::load_from(vars).unwrap();
        assert_eq!(config.timeout_secs, 5);
    }

    #[test]
    fn prefix_is_middleware() {
        assert_eq!(MiddlewareConfig::prefix(), "MIDDLEWARE_");
    }

    #[test]
    fn validate_always_succeeds() {
        let config = MiddlewareConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn load_from_loads_nested_cors_allowed_origins() {
        let vars = vec![(
            // 嵌套层级改用双下划线连接: CORS__ALLOWED_ORIGINS
            "MIDDLEWARE_CORS__ALLOWED_ORIGINS".to_string(),
            "https://example.com".to_string(),
        )];
        let config = MiddlewareConfig::load_from(vars).unwrap();

        assert_eq!(
            config.cors.allowed_origins,
            "https://example.com".to_string()
        );
    }

    #[test]
    fn load_from_loads_nested_rate_limit_params() {
        let vars = vec![
            (
                // RATE_LIMIT__MAX_REQUESTS
                "MIDDLEWARE_RATE_LIMIT__MAX_REQUESTS".to_string(),
                "200".to_string(),
            ),
            (
                // RATE_LIMIT__WINDOW_SECS
                "MIDDLEWARE_RATE_LIMIT__WINDOW_SECS".to_string(),
                "120".to_string(),
            ),
        ];
        let config = MiddlewareConfig::load_from(vars).unwrap();
        assert_eq!(config.rate_limit.max_requests, 200);
        assert_eq!(config.rate_limit.window_secs, 120);
    }
}
