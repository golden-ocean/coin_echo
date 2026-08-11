//! 应用组装：业务路由 + 中间件 + state 注入 → 可服务的 `Router`。

use std::sync::Arc;

use axum::Router;

use crate::AppState;
use crate::routes;

/// 组装最终应用。中间件对状态类型泛型化，挂在 `with_state` 之前之后
/// 都可以，这里选择在注入 state 之前挂，方便中间件将来如果需要访问
/// `AppState` 时不必再改动调用顺序。
pub fn build_app(state: Arc<AppState>) -> Router {
    let router: Router<Arc<AppState>> = Router::new().merge(routes::health::router());

    // 中间件挂载（platform-middleware 内部自行加载并降级配置）
    let router = platform_middleware::apply(router);

    // Router<Arc<AppState>> -> Router<()>
    router.with_state(state).merge(routes::openapi::router()) // 不需要状态，必须在 with_state 之后合并
}
