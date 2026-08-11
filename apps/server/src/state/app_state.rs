//! 应用共享状态。

use std::sync::Arc;

use platform_kernel::time::Clock;
// use platform_security::casbin::CasbinEnforcer; // casbin 暂未启用，见 infra.rs
use platform_security::jwt::JwtCodec;
use platform_security::password::PasswordHasher;

/// 以 `Arc<AppState>` 形式注入到 axum 路由。字段全部是 `platform-*`
/// crate 提供的"物理层"句柄，不含任何业务逻辑——业务逻辑属于各领域
/// `xxx-usecase`，这里只负责"造出这些句柄并让 handler 拿得到"。
pub struct AppState {
    pub db: platform_database::pg::PgPools,
    pub cache: platform_cache::redis::RedisPool,
    pub jwt: Arc<JwtCodec>,
    pub password_hasher: PasswordHasher,
    // pub casbin: CasbinEnforcer,
    pub clock: Arc<dyn Clock>,
}
