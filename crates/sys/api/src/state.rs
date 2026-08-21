use platform_kernel::time::Clock;
use sqlx::PgPool;
use std::sync::Arc;

use sys_application::ports::UnitOfWorkFactory;

#[derive(Clone)]
pub struct QueryState {
    pub reader_pool: PgPool,
}

#[derive(Clone)]
pub struct CommandState {
    pub uow_factory: Arc<dyn UnitOfWorkFactory>,
    pub clock: Arc<dyn Clock>,
}

#[derive(Clone)]
pub struct SysState {
    pub command_state: CommandState,
    pub query_state: QueryState,
}

impl SysState {
    pub fn new(
        reader_pool: sqlx::PgPool,
        uow_factory: Arc<dyn UnitOfWorkFactory>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            query_state: QueryState { reader_pool },
            command_state: CommandState { uow_factory, clock },
        }
    }
}

impl axum::extract::FromRef<SysState> for CommandState {
    fn from_ref(state: &SysState) -> Self {
        state.command_state.clone()
    }
}
impl axum::extract::FromRef<SysState> for QueryState {
    fn from_ref(state: &SysState) -> Self {
        state.query_state.clone()
    }
}
