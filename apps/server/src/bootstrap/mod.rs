//! 应用组合根：串联所有 `platform-*` crate 的初始化，构造 [`AppState`]，
//! 启动 HTTP 服务器。逻辑不在本文件——只做声明与导出，见各子模块。

mod infra;
mod run;
mod shutdown;

pub use run::run;
