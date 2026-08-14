use std::sync::Arc;

use iam_api::IamState;
use iam_infrastructure::{
    persistence::postgres::{PgStaffNoGenerator, PgUnitOfWorkFactory},
    security::Argon2PasswordHasher,
};

use crate::AppState;

pub fn build_iam_state(app_state: &AppState) -> IamState {
    IamState::new(
        app_state.pools.read.clone(),
        Arc::new(PgUnitOfWorkFactory::new(app_state.pools.write.clone())),
        Arc::new(Argon2PasswordHasher::new(app_state.password_hasher.clone())),
        Arc::new(PgStaffNoGenerator::new(app_state.pools.write.clone())),
        Arc::clone(&app_state.clock),
    )
}
