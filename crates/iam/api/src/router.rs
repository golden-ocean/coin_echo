use axum::{
    Router,
    routing::{delete, get, post, put},
};

use crate::{auth, permission, role, state::IamState, user};

/// 模块内部路由表

pub fn public_router(state: IamState) -> Router {
    Router::new()
        .route("/auth/login", post(auth::login))
        .route("/auth/refresh", post(auth::refresh_token))
        .with_state(state)
}

pub fn protected_router(state: IamState) -> Router {
    Router::new()
        .nest("/users", user_router())
        .nest("/roles", role_router())
        .nest("/permissions", permission_router())
        .with_state(state)
}

fn user_router() -> Router<IamState> {
    Router::new()
        .route("/", post(user::create_user))
        .route("/", get(user::page_user))
        .route("/{id}", put(user::update_user))
        .route("/{id}", delete(user::delete_user))
}

fn role_router() -> Router<IamState> {
    Router::new()
        .route("/", post(role::create_role))
        .route("/", get(role::page_role))
        .route("/{id}", put(role::update_role))
        .route("/{id}", delete(role::delete_role))
}

fn permission_router() -> Router<IamState> {
    Router::new()
        .route("/", post(permission::create_permission))
        .route("/", get(permission::list_permission))
        .route("/{id}", put(permission::update_permission))
        .route("/{id}", delete(permission::delete_permission))
}
