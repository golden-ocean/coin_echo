//! 中间件装配顺序。
//!
//! # 关于调用顺序
//!
//! `axum::Router::layer()` 语义：**后调用的 `.layer()` 是最外层、最先
//! 执行**。以下调用顺序是"期望执行顺序"的倒序。
//!
//! 期望的实际执行顺序（外→内）：
//! request_id → request_context → sensitive_headers → trace
//! → catch_panic → rate_limit → body_limit → timeout → compression
//! → cors → security_headers
//!
//! 顺序依赖关系说明：
//! - `request_context` 依赖 `request_id` 已经把 ID 写入请求头。
//! - `sensitive_headers` 必须在 `trace` 之外，否则访问日志打印的头
//!   信息看到的是未脱敏的原始值。
//! - `catch_panic`/`rate_limit`/`jwt`/`casbin`（jwt/casbin 不在这里，
//!   由业务路由自行挂载）在构造错误响应时都依赖
//!   `RequestContext::current()`，必须在 `request_context` 之内
//!   （更晚执行/更内层）。

use axum::Router;

use crate::config::MiddlewareConfig;
use crate::context::RequestContextLayer;
use crate::{
    body_limit, catch_panic, compression, cors, rate_limit, request_id, security_headers,
    sensitive_headers, timeout, trace,
};

pub fn apply(router: Router, cfg: &MiddlewareConfig) -> Router {
    let mut router = router;

    router = router.layer(security_headers::layer());
    router = router.layer(cors::layer(&cfg.cors));
    router = router.layer(compression::layer());
    router = router.layer(timeout::layer(cfg.timeout_secs));
    router = router.layer(body_limit::layer(cfg.body_limit_bytes));
    router = router.layer(rate_limit::layer(&cfg.rate_limit));
    router = router.layer(catch_panic::layer());
    router = router.layer(trace::layer::<axum::body::Body>());
    router = router.layer(sensitive_headers::request_layer());
    router = router.layer(sensitive_headers::response_layer());
    router = router.layer(RequestContextLayer);
    router = router.layer(request_id::layer());

    router
}
