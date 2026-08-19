//! 应用共享状态。
use std::sync::Arc;

use platform_kernel::time::Clock;
use platform_security::casbin::CasbinEnforcer;
use platform_security::{jwt::JwtCodec, password::PasswordHasher};

pub struct AppState {
    pub pools: platform_database::pg::PgPools,
    pub cache: platform_cache::redis::RedisPool,
    pub jwt: Arc<JwtCodec>,
    pub password_hasher: PasswordHasher,
    pub casbin: Arc<CasbinEnforcer>,
    pub clock: Arc<dyn Clock>,
}
