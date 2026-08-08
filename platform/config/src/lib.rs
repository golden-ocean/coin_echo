// lib.rs
//! 配置加载。
//!
//! 本 crate 只提供「从环境变量加载配置」的通用能力，以及
//! [`server::ServerConfig`]——服务器监听地址这类不属于任何具体基础设施的
//! 通用配置。具体基础设施（database/cache/telemetry/jwt/...）的配置结构体
//! 各自定义在对应 crate 内，见各 crate 文档。

mod server;
mod traits;

pub use server::ServerConfig;
pub use traits::{ConfigError, ConfigMeta, load_dotenv_if_present};
