//! 路由汇总 + 中间件挂载。
//!
//! 未来各领域 `xxx-api` crate 通过 `.merge(xxx_api::router(state.clone()))`
//! 在这里挂载。中间件本身不在这个 crate 里实现——全部来自
//! `platform-middleware`，这里只负责按配置调用它。

mod health;
mod openapi;

use std::sync::Arc;

use axum::Router;

use crate::AppState;
use platform_config::ConfigMeta;
use platform_middleware::MiddlewareConfig;

pub fn build(state: Arc<AppState>) -> Router {
    let router = Router::new()
        .route("/", axum::routing::get(|| async { "Hello, World!" }))
        .merge(health::router())
        .with_state(state)
        .merge(openapi::router());

    // 中间件配置错误不应阻止服务启动（与数据库等硬依赖不同），失败时
    // 落回默认值。
    let middleware_config = MiddlewareConfig::load().unwrap_or_else(|err| {
        tracing::warn!(%err, "中间件配置加载失败，使用默认值");
        MiddlewareConfig::default()
    });

    platform_middleware::apply(router, &middleware_config)
}
