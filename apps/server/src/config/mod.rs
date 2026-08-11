//! 仅本二进制使用的配置。跨 crate 复用的配置结构体（`DatabaseConfig`、
//! `JwtConfig` 等）分别定义在对应的 `platform-*` crate 里，不放这里。

mod server;

pub use server::ServerConfig;
