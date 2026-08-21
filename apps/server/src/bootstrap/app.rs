use std::sync::Arc;

use axum::Router;
use platform_security::context::SecurityContext;

use crate::{
    AppState,
    routes::{health, iam, openapi, sys},
};

pub fn build_app(state: Arc<AppState>) -> Router {
    let iam_routers = iam::build_routers(&state);
    let sys_routers = sys::build_routers(&state);
    // ---- v1 ----
    let v1_public = Router::new().nest("/iam", iam_routers.public);

    let v1_protected = Router::new()
        .nest("/iam", iam_routers.protected)
        .nest("/sys", sys_routers.protected)
        // 临时 todo
        .layer(axum::Extension(SecurityContext::new(uuid::Uuid::nil())));
    // .layer(platform_middleware::JwtAuthLayer::new(Arc::clone(
    //     &state.jwt,
    // )));

    let api_v1 = Router::new().merge(v1_public).merge(v1_protected);

    let app = Router::new()
        .merge(health::router().with_state(Arc::clone(&state)))
        .merge(openapi::router())
        .nest("/api/v1", api_v1);

    platform_middleware::apply(app)
}
