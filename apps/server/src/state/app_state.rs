//! 应用共享状态。

use std::sync::Arc;

use platform_kernel::time::Clock;
// use platform_security::casbin::CasbinEnforcer; // casbin 暂未启用，见 infra.rs
use platform_security::{jwt::JwtCodec, password::PasswordHasher};

/// 以 `Arc<AppState>` 形式注入到 axum 路由。字段全部是 `platform-*`
pub struct AppState {
    pub pools: platform_database::pg::PgPools,
    pub cache: platform_cache::redis::RedisPool,
    pub jwt: Arc<JwtCodec>,
    pub password_hasher: PasswordHasher,
    // pub casbin: CasbinEnforcer,
    pub clock: Arc<dyn Clock>,
}
