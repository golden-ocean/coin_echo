//! 应用组合根 crate。

mod bootstrap;
mod config;
mod routes;
mod state;

pub use bootstrap::run;
pub use state::AppState;
