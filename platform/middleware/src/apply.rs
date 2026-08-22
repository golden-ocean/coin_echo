use axum::Router;

use crate::{
    body_limit, catch_panic, config::MiddlewareConfig, context::RequestContextLayer, cors,
    request_id, security_headers, sensitive_headers, timeout, trace,
};
use platform_config::ConfigMeta;

pub fn apply<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let config = MiddlewareConfig::load().unwrap_or_else(|err| {
        tracing::warn!(%err, "中间件配置加载失败，使用默认值");
        MiddlewareConfig::default()
    });

    router
        .layer(security_headers::layer())
        .layer(cors::layer(&config.cors))
        .layer(timeout::layer(config.timeout_secs))
        .layer(body_limit::layer(config.body_limit_bytes))
        .layer(catch_panic::layer())
        // ----------------- 日志与脱敏核心层 -----------------
        .layer(sensitive_headers::response_layer()) // 响应先脱敏，再打日志
        .layer(trace::layer::<axum::body::Body>()) // 记录日志
        .layer(sensitive_headers::request_layer()) // 请求先脱敏，再传给日志
        // ----------------------------------------------------
        .layer(RequestContextLayer) // 注入请求上下文
        .layer(request_id::layer()) // 最最优先生成 Request ID
}
