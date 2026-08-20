use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, post, put},
};
use platform_middleware::PermissionLayer;
use platform_security::casbin::CasbinEnforcer;

use crate::{auth, permission, role, state::IamState, user};

/// 模块内部路由表

pub fn public_router(state: IamState) -> Router {
    Router::new()
        .route("/auth/login", post(auth::login))
        .route("/auth/refresh", post(auth::refresh_token))
        .with_state(state)
}

pub fn protected_router(state: IamState, enforcer: Arc<CasbinEnforcer>) -> Router {
    Router::new()
        .nest("/users", user_router(Arc::clone(&enforcer)))
        .nest("/roles", role_router(Arc::clone(&enforcer)))
        .nest("/permissions", permission_router(Arc::clone(&enforcer)))
        .with_state(state)
}

fn user_router(enforcer: Arc<CasbinEnforcer>) -> Router<IamState> {
    Router::new()
        .route("/", post(user::create_user))
        .route_layer(PermissionLayer::new(
            Arc::clone(&enforcer),
            "iam:user:create",
        ))
        .route("/", get(user::page_user))
        .route_layer(PermissionLayer::new(Arc::clone(&enforcer), "iam:user:page"))
        .route("/{id}", put(user::update_user))
        .route_layer(PermissionLayer::new(
            Arc::clone(&enforcer),
            "iam:user:update",
        ))
        .route("/{id}", delete(user::delete_user))
        .route_layer(PermissionLayer::new(
            Arc::clone(&enforcer),
            "iam:user:delete",
        ))
}

fn role_router(enforcer: Arc<CasbinEnforcer>) -> Router<IamState> {
    Router::new()
        .route("/", post(role::create_role))
        .route_layer(PermissionLayer::new(
            Arc::clone(&enforcer),
            "iam:role:create",
        ))
        .route("/", get(role::page_role))
        .route_layer(PermissionLayer::new(Arc::clone(&enforcer), "iam:role:page"))
        .route("/{id}", put(role::update_role))
        .route_layer(PermissionLayer::new(
            Arc::clone(&enforcer),
            "iam:role:update",
        ))
        .route("/{id}", delete(role::delete_role))
        .route_layer(PermissionLayer::new(
            Arc::clone(&enforcer),
            "iam:role:delete",
        ))
}

fn permission_router(enforcer: Arc<CasbinEnforcer>) -> Router<IamState> {
    Router::new()
        .route("/", post(permission::create_permission))
        .route_layer(PermissionLayer::new(
            Arc::clone(&enforcer),
            "iam:permission:create",
        ))
        .route("/", get(permission::list_permission))
        .route_layer(PermissionLayer::new(
            Arc::clone(&enforcer),
            "iam:permission:list",
        ))
        .route("/{id}", put(permission::update_permission))
        .route_layer(PermissionLayer::new(
            Arc::clone(&enforcer),
            "iam:permission:update",
        ))
        .route("/{id}", delete(permission::delete_permission))
        .route_layer(PermissionLayer::new(
            Arc::clone(&enforcer),
            "iam:permission:delete",
        ))
}
