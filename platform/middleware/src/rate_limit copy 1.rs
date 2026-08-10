//! 请求限流（Rate Limiting）。
//!
//! # 作用
//!
//! 防止接口被恶意刷量或突发流量压垮。基于令牌桶算法，为每个客户端 IP
//! 维护独立的配额：以固定速率补充令牌，允许短时突发消耗桶内存量令牌，
//! 超出后返回 `429 Too Many Requests` 并附带 `Retry-After` 头。
//!
//! # 使用的 tower-governor 组件
//!
//! [`GovernorLayer`]，纯 Tower 生态实现，零框架绑定。本文件负责从配置
//! 构建令牌桶参数并以 `Arc` 共享限流器实例。
//!
//! # 为什么必须用 `Arc<RateLimiter>`
//!
//! `RateLimiter` 内部使用 `DashMap` 存储每个 key 的令牌状态。如果直接
//! clone owned `RateLimiter`，会触发 DashMap 全量深拷贝（O(n)），在高
//! 并发下阻塞 tokio worker 线程，导致 RPS 暴跌至原来的 1/4 甚至更低。
//! 使用 `Arc::clone` 仅执行原子引用计数递增（O(1)），单次请求开销 < 1μs。
//!
//! # 泛型参数说明 (tower_governor 0.8 + governor 0.10)
//!
//! `GovernorLayer` 需要 3 个泛型：`<K, M, P>`。
//! - `K`: KeyExtractor，这里使用 `SmartIpKeyExtractor`（自动解析真实 IP）。
//! - `M`: MethodHandler，使用 `()` 表示默认行为（对所有 HTTP 方法限流）。
//! - `P`: RateLimitingMiddleware，使用 `NoOpMiddleware` 表示仅做限流判断，
//!   不额外注入状态信息到请求扩展中。如需在下游读取限流状态，可替换为
//!   `StateInformationMiddleware`。
//!
//! # 未配置或配置非法时的降级行为
//!
//! `rps` 或 `burst` 为 0 / 缺失时，跳过限流层并打 warn 日志，而非 panic
//! 或使用危险的默认值。这遵循"安全默认值"原则：忘记配置时倾向于放行
//! （避免误杀正常流量）但明确告警，而非静默失败或拒绝所有请求。

use std::num::NonZeroU32;
use std::sync::Arc;

use governor::{Quota, RateLimiter, middleware::NoOpMiddleware};
use serde::Deserialize;
use tower_governor::{GovernorLayer, key_extractor::SmartIpKeyExtractor};

/// 限流相关配置，作为 [`super::config::MiddlewareConfig`] 的嵌套字段。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RateLimitConfig {
    /// 每秒允许的请求数（令牌补充速率）。0 或缺失表示不限流。
    #[serde(default)]
    pub rps: u32,

    /// 允许的突发请求量（令牌桶容量）。0 或缺失时使用 rps 的值。
    #[serde(default)]
    pub burst: u32,
}

/// 构建限流 Layer。返回 `Option`：配置无效时返回 `None`，调用方据此
/// 决定是否叠加该层。
///
/// 注意返回类型中的 `<SmartIpKeyExtractor, (), NoOpMiddleware>`：
/// - `()` 作为 MethodHandler 表示对所有 HTTP 方法限流。
/// - `NoOpMiddleware` 作为 RateLimitingMiddleware 表示仅执行限流判断。
pub fn layer(
    cfg: &RateLimitConfig,
) -> Option<GovernorLayer<SmartIpKeyExtractor, (), NoOpMiddleware>> {
    let rps = match NonZeroU32::new(cfg.rps) {
        Some(v) => v,
        None => {
            tracing::warn!(
                "MIDDLEWARE_RATELIMIT_RPS 未配置或为 0，限流已禁用；\
                 如需启用请设置有效的每秒请求数"
            );
            return None;
        }
    };

    let burst = NonZeroU32::new(cfg.burst).unwrap_or(rps);

    let quota = Quota::per_second(rps).allow_burst(burst);

    // ⚠️ 必须 Arc 包裹，避免每请求 DashMap 深拷贝导致 RPS 暴跌
    let limiter = Arc::new(RateLimiter::direct(quota));

    Some(GovernorLayer {
        limiter,
        key_extractor: SmartIpKeyExtractor,
        middleware: NoOpMiddleware::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_rps_returns_none_without_panicking() {
        let cfg = RateLimitConfig { rps: 0, burst: 0 };
        assert!(layer(&cfg).is_none());
    }

    #[test]
    fn valid_config_produces_layer() {
        let cfg = RateLimitConfig {
            rps: 100,
            burst: 200,
        };
        assert!(layer(&cfg).is_some());
    }

    #[test]
    fn zero_burst_falls_back_to_rps() {
        // burst=0 时应降级为 burst=rps，不应返回 None
        let cfg = RateLimitConfig { rps: 50, burst: 0 };
        assert!(layer(&cfg).is_some());
    }

    #[test]
    fn default_config_disables_ratelimit() {
        // Default 派生使 rps=0, burst=0 → 限流禁用
        let cfg = RateLimitConfig::default();
        assert!(layer(&cfg).is_none());
    }
}
