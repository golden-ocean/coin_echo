//! 结构化日志的配置与初始化，以及跨调用点统一的错误观测记录。
//!
//! # 定位
//!
//! 当前只覆盖日志（`tracing` + `tracing-subscriber`）。分布式追踪
//! （OTLP/Jaeger 等）留待真正出现跨服务调用链路时再引入——见
//! [`config::TelemetryConfig`] 里已预留但默认关闭的 OTel 相关字段，以及
//! [`init`] 模块里对应的桩函数。提前预留字段和接入点，是为了让"以后接
//! OTel"这件事局限在 `init.rs` 内部改动，不需要触及任何调用方代码。
//!
//! # 统一的错误观测记录
//!
//! [`record::ErrorObservation`] 是"如何把一个 `impl ErrorMeta` 记录成
//! 可观测数据"的唯一定义处。中间件（`catch_panic`/`rate_limit`/`auth`
//! 等）不应该各自手写 `tracing::warn!(code = .., kind = .., ...)`，而是
//! 统一调用 [`record::record_error`]——这样字段名、字段集合只在一处维护；
//! 未来接入 OTel 时，只需要在这一处把 `tracing::event!` 换成同时携带
//! span attributes 的等价写法（或依赖 `tracing-opentelemetry` 桥接层
//! 自动转换现有字段），调用点完全不用动。
//!
//! # 模块划分
//!
//! - [`config`] 配置结构体，实现 [`platform_config::ConfigMeta`]
//! - [`error`] 错误类型
//! - [`record`] 统一错误观测记录
//! - `init` 组装并安装全局订阅器

mod config;
mod error;
mod init;
mod record;

pub use config::{LogFormat, TelemetryConfig, TelemetryConfigError};
pub use error::TelemetryError;
pub use init::{TelemetryGuard, init};
pub use record::{ErrorObservation, record_error};
