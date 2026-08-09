//! 基础设施初始化：database / cache / security。
//!
//! 各 `XxxConfig::load()` 内部已经通过 [`platform_config::ConfigMeta`]
//! 自动完成"反序列化 + 语义校验"，这里不需要再手动调用 `.validate()`。

use std::sync::Arc;

use platform_cache::redis::{RedisConfig, RedisPool};
use platform_config::ConfigMeta;
use platform_database::pg::{PgDatabaseConfig, PgPools};
use platform_kernel::time::{Clock, SystemClock};
use platform_security::casbin::{CasbinConfig, CasbinEnforcer};
use platform_security::jwt::{JwtCodec, JwtConfig};
use platform_security::password::{PasswordConfig, PasswordHasher};

use crate::state::AppState;

pub async fn build_state() -> anyhow::Result<AppState> {
    let database_cfg = PgDatabaseConfig::load()?;
    let db = PgPools::connect(&database_cfg).await?;
    // platform_database::run_migrations(&db.write).await?;
    tracing::info!("数据库连接池已就绪");

    let cache_cfg = RedisConfig::load()?;
    let cache = RedisPool::connect(&cache_cfg)?;
    tracing::info!("缓存连接池已就绪");

    let jwt_cfg = JwtConfig::load()?;
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let jwt = Arc::new(JwtCodec::new(&jwt_cfg, Arc::clone(&clock)));

    let password_cfg = PasswordConfig::load()?;
    let password_hasher = PasswordHasher::new(&password_cfg)?;

    // let casbin_cfg = CasbinConfig::load()?;
    // let casbin = CasbinEnforcer::new(&casbin_cfg).await?;
    // tracing::info!("安全组件（jwt/password/casbin）已就绪");

    Ok(AppState {
        db,
        cache,
        jwt,
        password_hasher,
        // casbin,
        clock,
    })
}
