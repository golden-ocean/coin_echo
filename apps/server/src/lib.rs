//! 应用组合根 crate。
//!
//! 本 crate 不包含业务逻辑，只负责把 `platform-*` 基础设施 crate 组装
//! 成一个可运行的 HTTP 服务：读配置 → 建连接池/安全组件 → 组路由 →
//! 挂载 `platform-middleware` 提供的中间件栈 → 启动服务器。

mod bootstrap;
mod config;
mod routes;
mod state;

pub use bootstrap::run;
pub use state::AppState;
