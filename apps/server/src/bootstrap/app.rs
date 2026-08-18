use axum::Router;
use platform_middleware::CurrentUser;
use std::sync::Arc;

use crate::{
    AppState,
    routes::{health, iam, openapi},
};

pub fn build_app(state: Arc<AppState>) -> Router {
    let public: Router = Router::new()
        .merge(health::router().with_state(Arc::clone(&state)))
        .merge(openapi::router())
        .nest("/api/v1", iam::public_router(&state));

    let protected: Router = Router::new()
        .nest("/api/v1", iam::protected_router(&state))
        .layer(axum::Extension(CurrentUser {
            id: uuid::Uuid::now_v7(),
        })); // 伪造身份，替代真实的 JWT 校验流程
    // .layer(platform_middleware::JwtAuthLayer::new(Arc::clone(
    //     &state.jwt,
    // )));

    // .layer(platform_middleware::CasbinLayer::new(Arc::clone(&state.casbin)));

    let merged: Router = public.merge(protected);
    platform_middleware::apply(merged)
}
