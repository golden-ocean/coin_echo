//! 错误契约。
//!
//! # 设计要点
//!
//! 全项目的错误处理围绕 [`ErrorMeta`] 展开：每一层定义自己的错误枚举（用
//! `thiserror`），并实现 [`ErrorMeta`] 声明其**语义**；传输层据此把错误渲染成
//! 对外表示（HTTP 下是 RFC 9457 `application/problem+json`）。
//!
//! # 为什么 [`ErrorKind`] 不带 HTTP 状态码
//!
//! 一旦在此处出现 `fn status(&self) -> u16`，HTTP 语义就渗进了领域层。之后新增
//! 后台任务、CLI 或 gRPC 入口时，它们都得被迫理解「404 是什么意思」。
//!
//! 因此这里只描述**语义类别**，状态码映射是 `web`层 的职责。同一个
//! [`ErrorKind::Conflict`] 在 HTTP 下是 409，在 gRPC 下是 `ABORTED`，互不干扰。
//!
//! # 典型用法
//!
//! ```
//! use platform_kernel::error::{ErrorKind, ErrorMeta};
//!
//! #[derive(Debug, thiserror::Error)]
//! enum DomainError {
//!     #[error("用户不存在")]
//!     UserNotFound,
//!     #[error("余额不足")]
//!     InsufficientBalance,
//! }
//!
//! impl ErrorMeta for DomainError {
//!     fn kind(&self) -> ErrorKind {
//!         match self {
//!             Self::UserNotFound => ErrorKind::NotFound,
//!             Self::InsufficientBalance => ErrorKind::Conflict,
//!         }
//!     }
//!
//!     fn code(&self) -> &'static str {
//!         match self {
//!             Self::UserNotFound => "user.not_found",
//!             Self::InsufficientBalance => "wallet.insufficient_balance",
//!         }
//!     }
//! }
//! ```

use std::{borrow::Cow, fmt};

/// 全局错误分类 (对应 HTTP 状态码与分类行为)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum ErrorKind {
    /// 入参不合法：格式错误、越界、必填缺失、业务前置校验未通过。
    Validation,
    /// 身份未知：缺少凭证，或凭证无效、已过期。
    Unauthenticated,
    /// 身份已知但无权限执行该操作。
    Forbidden,
    /// 目标资源不存在，或对当前调用者不可见。
    NotFound,
    /// 与资源当前状态冲突：唯一约束、乐观锁版本不匹配、状态机非法迁移。
    Conflict,
    /// 触发速率限制或配额上限。
    Exhausted,
    /// 处理超时。区别于 [`Unavailable`](Self::Unavailable)：请求可能已生效，
    /// 重试前需考虑幂等性。
    Timeout,
    /// 依赖不可用：数据库连接失败、下游服务熔断、正在停机排空。
    Unavailable,
    /// 未预期的内部故障。对外必须脱敏，只暴露 `trace_id`。
    Internal,
}

impl ErrorKind {
    // /// 映射 HTTP 状态码 (解耦 Web 框架)
    // pub fn status_code(&self) -> u16 {
    //     match self {
    //         Self::BadRequest => 400,
    //         Self::Unauthorized => 401,
    //         Self::Forbidden => 403,
    //         Self::NotFound => 404,
    //         Self::Conflict => 409,
    //         Self::UnprocessableEntity => 422,
    //         Self::TooManyRequests => 429,
    //         Self::Internal => 500,
    //         Self::Upstream => 502,
    //     }
    // }

    // /// RFC 9457 Title 描述
    // pub fn title(&self) -> &'static str {
    //     match self {
    //         Self::BadRequest => "Bad Request",
    //         Self::Unauthorized => "Unauthorized",
    //         Self::Forbidden => "Forbidden",
    //         Self::NotFound => "Not Found",
    //         Self::Conflict => "Conflict",
    //         Self::UnprocessableEntity => "Unprocessable Entity",
    //         Self::TooManyRequests => "Too Many Requests",
    //         Self::Internal => "Internal Server Error",
    //         Self::Upstream => "Upstream Service Error",
    //     }
    // }

    /// 稳定的机器可读名称，用于日志字段与指标标签。
    ///
    /// 该取值是对外契约的一部分，一经发布不得修改。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Validation => "validation",
            Self::Unauthenticated => "unauthenticated",
            Self::Forbidden => "forbidden",
            Self::NotFound => "not_found",
            Self::Conflict => "conflict",
            Self::Exhausted => "exhausted",
            Self::Timeout => "timeout",
            Self::Unavailable => "unavailable",
            Self::Internal => "internal",
        }
    }

    /// 责任是否在调用方。
    ///
    /// 决定两件事：日志级别（调用方问题记 `warn`，服务端问题记 `error`）、
    /// 以及是否计入服务端错误率 SLO。把参数校验失败计进 SLO 会让告警彻底失真。
    #[must_use]
    pub const fn is_caller_fault(self) -> bool {
        matches!(
            self,
            Self::Validation
                | Self::Unauthenticated
                | Self::Forbidden
                | Self::NotFound
                | Self::Conflict
                | Self::Exhausted
        )
    }

    /// 原样重试是否有意义（默认判断，可被 [`ErrorMeta::retryable`] 覆盖）。
    ///
    /// 注意 [`Timeout`](Self::Timeout) 归为可重试，但调用方必须自行保证幂等
    /// —— 超时意味着结果未知，请求可能已经生效。
    #[must_use]
    pub const fn retryable_by_default(self) -> bool {
        matches!(self, Self::Timeout | Self::Unavailable | Self::Exhausted)
    }

    /// 对外暴露细节是否安全。
    ///
    /// 服务端错误的 `detail` 常包含 SQL 片段、内网地址、字段名等实现信息，直接
    /// 回给客户端等于免费提供攻击面测绘。
    #[must_use]
    pub const fn is_detail_safe_to_expose(self) -> bool {
        self.is_caller_fault()
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 字段级校验错误。
///
/// 用于把「哪个字段错了、错在哪」结构化地传给客户端，避免前端靠解析错误文案
/// 来定位字段 —— 那样文案一改客户端就崩。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct FieldError {
    /// 字段路径，嵌套结构用点号连接，数组用下标：`items.0.quantity`。
    pub field: Cow<'static, str>,
    /// 稳定的错误代码，如 `required` / `out_of_range` / `already_taken`。
    pub code: &'static str,
    /// 面向人的说明。可能随文案调整而变化，客户端不得依赖其内容做逻辑判断。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl FieldError {
    /// 构造一个字段错误。
    pub fn new(field: impl Into<Cow<'static, str>>, code: &'static str) -> Self {
        Self {
            field: field.into(),
            code,
            message: None,
        }
    }

    /// 附加面向人的说明。
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}
