//! 路由汇总 + 中间件挂载。
//!
//! 未来各领域 `xxx-api` crate 通过 `.merge(xxx_api::router())` 在这里
//! 挂载。中间件本身不在这个 crate 里实现，全部来自
//! `platform-middleware`，这里只负责调用 `bootstrap::middleware::apply`。
//!
//! # 挂载顺序（重要）
//!
//! 需要访问 `AppState` 的路由（如 `health`）必须在 `.with_state(state)`
//! 之前 merge；不需要状态的路由（如 `openapi`）必须在
//! `.with_state(state)` 之后 merge——两者泛型参数不同
//! （`Router<Arc<AppState>>` vs `Router<()>`），顺序错了会直接编译不过。

mod health;
mod openapi;

use std::sync::Arc;

use axum::Router;

use crate::AppState;

pub fn build(state: Arc<AppState>) -> Router {
    let router: Router<Arc<AppState>> = Router::new().merge(health::router());

    // 中间件对状态类型泛型化，挂在 with_state 之前之后都可以，
    // 这里选择在注入 state 之前挂，方便中间件里将来如果需要访问
    // AppState 时不必再改动这里的调用顺序。
    let router = platform_middleware::apply(router);

    router
        .with_state(state) // Router<Arc<AppState>> -> Router<()>
        .merge(openapi::router()) // 不需要状态，必须在这一步之后合并
}
