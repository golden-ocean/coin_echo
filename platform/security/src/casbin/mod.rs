//! 基于 Casbin 的访问控制。

mod config;
mod enforcer;
mod error;

pub use config::{CasbinConfig, CasbinConfigError};
pub use enforcer::{CasbinEnforcer, RBAC_MODEL};
pub use error::CasbinError;
