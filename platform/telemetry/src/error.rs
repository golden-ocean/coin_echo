//! 遥测初始化错误。

/// 遥测（日志）初始化过程中的错误。
///
/// 这类错误只发生在进程启动阶段，不属于请求级错误，因此不需要实现
/// `platform_kernel::error::ErrorMeta`——没有 HTTP 客户端在等着接收它，
/// 失败应直接终止启动进程（fail fast），而不是渲染成响应体。
#[derive(Debug, thiserror::Error)]
pub enum TelemetryError {
    /// 全局订阅器已被设置过（进程内只能设置一次）。
    #[error("全局日志订阅器已被设置，不能重复初始化")]
    AlreadyInitialized,

    /// 日志过滤指令（如 `sqlx=warn,app=debug`）语法非法。
    #[error("日志过滤指令无效：{0}")]
    InvalidFilter(String),
}
