use axum::{
    Router,
    routing::{delete, get, post, put},
};

use crate::{role, state::IamState, user};

/// 模块内部路由表
pub fn router(state: IamState) -> Router {
    Router::new()
        .nest("/users", user_router())
        .nest("/roles", role_router())
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
