use platform_kernel::time::Clock;
use sqlx::PgPool;
use std::sync::Arc;

use iam_application::ports::{
    PasswordHasher, PolicyService, StaffNoGenerator, TokenService, UnitOfWorkFactory,
};

#[derive(Clone)]
pub struct QueryState {
    pub reader_pool: PgPool,
}

#[derive(Clone)]
pub struct CommandState {
    pub uow_factory: Arc<dyn UnitOfWorkFactory>,
    pub password_hasher: Arc<dyn PasswordHasher>,
    pub staff_no_generator: Arc<dyn StaffNoGenerator>,
    pub policy_service: Arc<dyn PolicyService>,
    pub token_service: Arc<dyn TokenService>,
    pub clock: Arc<dyn Clock>,
}

#[derive(Clone)]
pub struct IamState {
    pub command_state: CommandState,
    pub query_state: QueryState,
}

impl IamState {
    pub fn new(
        reader_pool: sqlx::PgPool,
        uow_factory: Arc<dyn UnitOfWorkFactory>,
        password_hasher: Arc<dyn PasswordHasher>,
        staff_no_generator: Arc<dyn StaffNoGenerator>,
        clock: Arc<dyn Clock>,
        policy_service: Arc<dyn PolicyService>,
        token_service: Arc<dyn TokenService>,
    ) -> Self {
        Self {
            query_state: QueryState { reader_pool },
            command_state: CommandState {
                uow_factory,
                password_hasher,
                staff_no_generator,
                clock,
                policy_service,
                token_service,
            },
        }
    }
}

impl axum::extract::FromRef<IamState> for CommandState {
    fn from_ref(state: &IamState) -> Self {
        state.command_state.clone()
    }
}
impl axum::extract::FromRef<IamState> for QueryState {
    fn from_ref(state: &IamState) -> Self {
        state.query_state.clone()
    }
}
