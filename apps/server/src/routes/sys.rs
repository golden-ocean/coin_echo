use std::sync::Arc;

use axum::Router;

use crate::AppState;

/// 组装 Sys 域需要的全部依赖，只在应用启动时调用一次。
fn build_sys_state(app_state: &AppState) -> sys_api::state::SysState {
    sys_api::state::SysState::new(
        app_state.pools.read.clone(),
        Arc::new(sys_infrastructure::persistence::PgUnitOfWorkFactory::new(
            app_state.pools.write.clone(),
        )),
        Arc::clone(&app_state.clock),
    )
}

/// Sys 域的公开/受保护路由。两者共享同一份 SysState 快照

pub struct Routers {
    pub protected: Router,
}

pub fn build_routers(app_state: &AppState) -> Routers {
    let state = build_sys_state(app_state);
    let enforcer = Arc::clone(&app_state.casbin_enforcer);

    Routers {
        protected: sys_api::router::protected_router(state, enforcer),
    }
}
