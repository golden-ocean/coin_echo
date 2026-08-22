use std::sync::Arc;

use axum::Router;

use crate::AppState;

fn build_org_state(app_state: &AppState) -> org_api::state::OrgState {
    org_api::state::OrgState::new(
        app_state.pools.read.clone(),
        Arc::new(org_infrastructure::persistence::PgUnitOfWorkFactory::new(
            app_state.pools.write.clone(),
        )),
        Arc::new(org_infrastructure::persistence::PgMembershipChecker::new(
            app_state.pools.write.clone(),
        )),
        Arc::clone(&app_state.clock),
    )
}

pub struct Routers {
    pub protected: Router,
}

pub fn build_routers(app_state: &AppState) -> Routers {
    let state = build_org_state(app_state);
    let enforcer = Arc::clone(&app_state.casbin_enforcer);

    Routers {
        protected: org_api::router::protected_router(state, enforcer),
    }
}
