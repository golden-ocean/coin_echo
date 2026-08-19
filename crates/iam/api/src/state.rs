use platform_kernel::time::Clock;
use platform_security::casbin::CasbinEnforcer;
use sqlx::PgPool;
use std::sync::Arc;

use iam_application::ports::{PasswordHasher, StaffNoGenerator, TokenService, UnitOfWorkFactory};

#[derive(Clone)]
pub struct QueryState {
    pub reader_pool: PgPool,
    pub enforcer: Arc<CasbinEnforcer>,
}

#[derive(Clone)]
pub struct CommandState {
    pub uow_factory: Arc<dyn UnitOfWorkFactory>,
    pub password_hasher: Arc<dyn PasswordHasher>,
    pub staff_no_generator: Arc<dyn StaffNoGenerator>,
    pub clock: Arc<dyn Clock>,
    pub token_service: Arc<dyn TokenService>,
    pub enforcer: Arc<CasbinEnforcer>,
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
        token_service: Arc<dyn TokenService>,
        enforcer: Arc<CasbinEnforcer>,
    ) -> Self {
        Self {
            query_state: QueryState {
                reader_pool,
                enforcer: enforcer.clone(),
            },
            command_state: CommandState {
                uow_factory,
                password_hasher,
                staff_no_generator,
                clock,
                token_service,
                enforcer,
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
