use std::sync::Arc;

use crate::AppState;
use axum::Router;
use iam_infrastructure::{
    persistence::postgres::{PgStaffNoGenerator, PgUnitOfWorkFactory},
    security::{Argon2PasswordHasher, JwtTokenService},
};

fn build_iam_state(app_state: &AppState) -> iam_api::IamState {
    iam_api::IamState::new(
        app_state.pools.read.clone(),
        Arc::new(PgUnitOfWorkFactory::new(app_state.pools.write.clone())),
        Arc::new(Argon2PasswordHasher::new(app_state.password_hasher.clone())),
        Arc::new(PgStaffNoGenerator::new(app_state.pools.write.clone())),
        Arc::clone(&app_state.clock),
        Arc::new(JwtTokenService::new(app_state.jwt.clone())),
    )
}

pub fn public_router(app_state: &AppState) -> Router {
    iam_api::public_router(build_iam_state(app_state))
}

pub fn protected_router(app_state: &AppState) -> Router {
    iam_api::protected_router(build_iam_state(app_state))
}
