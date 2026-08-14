//! 应用组装：业务路由 + 中间件 + state 注入 → 可服务的 `Router`。

use std::sync::Arc;

use axum::Router;
use serde_json::json;

use crate::AppState;
use crate::routes;

/// 组装最终应用。中间件对状态类型泛型化，挂在 `with_state` 之前之后
pub fn build_app(state: Arc<AppState>) -> Router {
    let iam_state = crate::state::build_iam_state(&state);
    let iam_router = iam_api::router(iam_state);

    let base_router: Router<Arc<AppState>> = Router::new().merge(routes::health::router());

    let app_router = platform_middleware::apply(base_router).with_state(state);

    app_router
        .merge(iam_router)
        .merge(routes::openapi::router())
        .merge(scalar_api_reference::axum::router(
            "/scalar",
            &json!({
                "url": "/openapi.json",
            }),
        ))
}
