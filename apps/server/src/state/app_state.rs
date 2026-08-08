use std::sync::Arc;

use platform_database::PgPools;

/// 全局唯一状态
#[derive(Clone)]
pub struct AppState {
    // pub iam: IamState,
}

impl AppState {
    pub fn new(pools: PgPools) -> Self {
        Self {
            // iam: IamState::new(
            //     pool.clone(),
            //     Arc::new(PgUnitOfWorkFactory::new(pool.clone())),
            //     Arc::new(Argon2PasswordHasher::default()),
            //     Arc::new(PgStaffNoGenerator::new(pool.clone())),
            //     Arc::new(SystemClock),
            // ),
        }
    }
}
