use axum::Router;
use std::sync::Arc;

use crate::{
    AppState,
    routes::{health, iam, openapi},
};

pub fn build_app(state: Arc<AppState>) -> Router {
    // 公开路由：不需要认证，但仍需要基础设施中间件
    let public: Router = Router::new()
        .merge(health::router().with_state(Arc::clone(&state)))
        .merge(openapi::router());

    // 受保护路由：需要认证+鉴权，挂载 jwt/casbin，仅作用于这一组
    let protected: Router = Router::new().nest("/api/v1", iam::router(&state));
    // .layer(platform_middleware::jwt::layer(/* ... */))
    // .layer(platform_middleware::casbin::layer(/* ... */));

    let merged: Router = public.merge(protected);

    // 统一包住整个应用，公开和受保护路由都要有
    platform_middleware::apply(merged)
}
