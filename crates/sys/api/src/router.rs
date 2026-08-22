use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, post, put},
};
use platform_middleware::PermissionLayer;
use platform_security::casbin::CasbinEnforcer;

use crate::{dictionary, dictionary_item, policy, state::SysState};

/// 模块内部路由表
pub fn protected_router(state: SysState, enforcer: Arc<CasbinEnforcer>) -> Router {
    Router::new()
        .nest("/dictionaries", dictionary_router(Arc::clone(&enforcer)))
        .nest(
            "/dictionary/items",
            dictionary_item_router(Arc::clone(&enforcer)),
        )
        .with_state(state)
}

fn dictionary_router(enforcer: Arc<CasbinEnforcer>) -> Router<SysState> {
    Router::new()
        .route(
            "/",
            post(dictionary::create_dictionary).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::sys_policy::dictionary::CREATE,
            )),
        )
        .route(
            "/",
            get(dictionary::list_dictionary).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::sys_policy::dictionary::LIST,
            )),
        )
        .route(
            "/{id}",
            put(dictionary::update_dictionary).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::sys_policy::dictionary::UPDATE,
            )),
        )
        .route(
            "/{id}",
            delete(dictionary::delete_dictionary).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::sys_policy::dictionary::DELETE,
            )),
        )
}

fn dictionary_item_router(enforcer: Arc<CasbinEnforcer>) -> Router<SysState> {
    Router::new()
        .route(
            "/",
            post(dictionary_item::create_dictionary_item).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::sys_policy::dictionary_item::CREATE,
            )),
        )
        .route(
            "/",
            get(dictionary_item::page_dictionary_item).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::sys_policy::dictionary_item::PAGE,
            )),
        )
        .route(
            "/{id}/display",
            put(dictionary_item::update_dictionary_item).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::sys_policy::dictionary_item::UPDATE,
            )),
        )
        .route(
            "/{id}",
            delete(dictionary_item::delete_dictionary_item).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::sys_policy::dictionary_item::DELETE,
            )),
        )
}

