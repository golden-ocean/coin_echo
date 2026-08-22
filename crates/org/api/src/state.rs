use std::sync::Arc;

use platform_kernel::time::Clock;
use sqlx::PgPool;

use org_application::ports::{MembershipChecker, UnitOfWorkFactory};

#[derive(Clone)]
pub struct QueryState {
    pub reader_pool: PgPool,
}

#[derive(Clone)]
pub struct CommandState {
    pub uow_factory: Arc<dyn UnitOfWorkFactory>,
    pub membership_checker: Arc<dyn MembershipChecker>,
    pub clock: Arc<dyn Clock>,
}

#[derive(Clone)]
pub struct OrgState {
    pub command_state: CommandState,
    pub query_state: QueryState,
}

impl OrgState {
    pub fn new(
        reader_pool: sqlx::PgPool,
        uow_factory: Arc<dyn UnitOfWorkFactory>,
        membership_checker: Arc<dyn MembershipChecker>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            query_state: QueryState { reader_pool },
            command_state: CommandState {
                uow_factory,
                membership_checker,
                clock,
            },
        }
    }
}

impl axum::extract::FromRef<OrgState> for CommandState {
    fn from_ref(state: &OrgState) -> Self {
        state.command_state.clone()
    }
}
impl axum::extract::FromRef<OrgState> for QueryState {
    fn from_ref(state: &OrgState) -> Self {
        state.query_state.clone()
    }
}
