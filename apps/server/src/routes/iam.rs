use std::sync::Arc;

use axum::Router;

use iam_infrastructure::{
    persistence::postgres::{PgStaffNoGenerator, PgUnitOfWorkFactory},
    security::{Argon2PasswordHasher, JwtTokenService},
};

use crate::AppState;

/// 组装 IAM 域需要的全部依赖，只在应用启动时调用一次。
/// IamState 内部全是 Arc，Clone 成本可忽略，public_router/protected_router
/// 共享同一份，而不是各自重新构造一遍 PgUnitOfWorkFactory /
/// Argon2PasswordHasher / PgStaffNoGenerator / JwtTokenService。
fn build_iam_state(app_state: &AppState) -> iam_api::IamState {
    iam_api::IamState::new(
        app_state.pools.read.clone(),
        Arc::new(PgUnitOfWorkFactory::new(app_state.pools.write.clone())),
        Arc::new(Argon2PasswordHasher::new(app_state.password_hasher.clone())),
        Arc::new(PgStaffNoGenerator::new(app_state.pools.write.clone())),
        Arc::clone(&app_state.clock),
        Arc::new(JwtTokenService::new(app_state.jwt.clone())),
        Arc::clone(&app_state.casbin),
    )
}

/// IAM 域的公开/受保护路由。两者共享同一份 IamState 快照

pub struct Routers {
    pub public: Router,
    pub protected: Router,
}

pub fn build_routers(app_state: &AppState) -> Routers {
    let state = build_iam_state(app_state);

    Routers {
        public: iam_api::public_router(state.clone()),
        protected: iam_api::protected_router(state),
    }
}
