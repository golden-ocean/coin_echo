use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, post, put},
};
use platform_middleware::PermissionLayer;
use platform_security::casbin::CasbinEnforcer;

use crate::{organization, policy, position, state::OrgState};

/// 模块内部路由表
pub fn protected_router(state: OrgState, enforcer: Arc<CasbinEnforcer>) -> Router {
    Router::new()
        .nest("/organizations", organization_router(Arc::clone(&enforcer)))
        .nest("/positions", position_router(Arc::clone(&enforcer)))
        .with_state(state)
}

fn organization_router(enforcer: Arc<CasbinEnforcer>) -> Router<OrgState> {
    Router::new()
        .route(
            "/",
            post(organization::create_organization).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::org_policy::organization::CREATE,
            )),
        )
        .route(
            "/",
            get(organization::list_organization).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::org_policy::organization::LIST,
            )),
        )
        .route(
            "/{id}",
            put(organization::update_organization).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::org_policy::organization::UPDATE,
            )),
        )
        .route(
            "/{id}/move",
            put(organization::move_organization).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::org_policy::organization::MOVE,
            )),
        )
        .route(
            "/{id}",
            delete(organization::delete_organization).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::org_policy::organization::DELETE,
            )),
        )
}

fn position_router(enforcer: Arc<CasbinEnforcer>) -> Router<OrgState> {
    Router::new()
        .route(
            "/",
            post(position::create_position).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::org_policy::position::CREATE,
            )),
        )
        .route(
            "/",
            get(position::list_position).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::org_policy::position::LIST,
            )),
        )
        .route(
            "/{id}",
            put(position::update_position).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::org_policy::position::UPDATE,
            )),
        )
        .route(
            "/{id}",
            delete(position::delete_position).route_layer(PermissionLayer::new(
                Arc::clone(&enforcer),
                policy::org_policy::position::DELETE,
            )),
        )
}
