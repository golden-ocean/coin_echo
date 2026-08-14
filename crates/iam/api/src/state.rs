use platform_kernel::time::Clock;
use sqlx::PgPool;
use std::sync::Arc;

use iam_application::ports::{PasswordHasher, StaffNoGenerator, UnitOfWorkFactory};

#[derive(Clone)]
pub struct QueryState {
    pub reader_pool: PgPool,
}

#[derive(Clone)]
pub struct CommandState {
    pub uow_factory: Arc<dyn UnitOfWorkFactory>,
    pub password_hasher: Arc<dyn PasswordHasher>,
    pub staff_no_generator: Arc<dyn StaffNoGenerator>,
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
    ) -> Self {
        Self {
            query_state: QueryState { reader_pool },
            command_state: CommandState {
                uow_factory,
                password_hasher,
                staff_no_generator,
                clock,
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
