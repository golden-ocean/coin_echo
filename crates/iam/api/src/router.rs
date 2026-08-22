use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, post, put},
};
use platform_middleware::PermissionLayer;
use platform_security::casbin::CasbinEnforcer;

use crate::{auth, permission, policy, role, state::IamState, user};

/// 模块公开路由表
pub fn public_router(state: IamState) -> Router {
    Router::new()
        .route("/auth/login", post(auth::login))
        .route("/auth/refresh", post(auth::refresh_token))
        .with_state(state)
}

/// 模块受保护路由表
pub fn protected_router(state: IamState, enforcer: Arc<CasbinEnforcer>) -> Router {
    Router::new()
        .nest("/users", user_router(Arc::clone(&enforcer)))
        .nest("/roles", role_router(Arc::clone(&enforcer)))
        .nest("/permissions", permission_router(Arc::clone(&enforcer)))
        .with_state(state)
}

fn user_router(enforcer: Arc<CasbinEnforcer>) -> Router<IamState> {
    Router::new()
        .route(
            "/",
            post(user::create_user).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::iam_policy::user::CREATE,
            )),
        )
        .route(
            "/",
            get(user::page_user).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::iam_policy::user::PAGE,
            )),
        )
        .route(
            "/{id}",
            put(user::update_user).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::iam_policy::user::UPDATE,
            )),
        )
        .route(
            "/{id}",
            delete(user::delete_user).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::iam_policy::user::DELETE,
            )),
        )
}

fn role_router(enforcer: Arc<CasbinEnforcer>) -> Router<IamState> {
    Router::new()
        .route(
            "/",
            post(role::create_role).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::iam_policy::role::CREATE,
            )),
        )
        .route(
            "/",
            get(role::page_role).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::iam_policy::role::PAGE,
            )),
        )
        .route(
            "/{id}",
            put(role::update_role).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::iam_policy::role::UPDATE,
            )),
        )
        .route(
            "/{id}",
            delete(role::delete_role).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::iam_policy::role::DELETE,
            )),
        )
}

fn permission_router(enforcer: Arc<CasbinEnforcer>) -> Router<IamState> {
    Router::new()
        .route(
            "/",
            post(permission::create_permission).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::iam_policy::permission::CREATE,
            )),
        )
        .route(
            "/",
            get(permission::list_permission).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::iam_policy::permission::LIST,
            )),
        )
        .route(
            "/{id}",
            put(permission::update_permission).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::iam_policy::permission::UPDATE,
            )),
        )
        .route(
            "/{id}",
            delete(permission::delete_permission).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::iam_policy::permission::DELETE,
            )),
        )
}
