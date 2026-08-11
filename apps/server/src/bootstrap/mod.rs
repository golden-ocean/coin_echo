//! 应用启动流程的组合根。逻辑不写在这里，只做模块声明与导出，
//! 具体实现见各子模块。

mod infra;
mod run;
mod shutdown;

pub use run::run;
