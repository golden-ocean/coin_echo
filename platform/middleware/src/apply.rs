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
        // .layer(compression::layer())        // 未启用
        .layer(timeout::layer(config.timeout_secs))
        .layer(body_limit::layer(config.body_limit_bytes))
        // .layer(rate_limit::layer(&config.rate_limit))  // 未启用
        .layer(catch_panic::layer())
        .layer(trace::layer::<axum::body::Body>())
        .layer(sensitive_headers::request_layer())
        .layer(sensitive_headers::response_layer())
        .layer(RequestContextLayer)
        .layer(request_id::layer())
}
