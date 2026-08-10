use std::borrow::Cow;

use crate::error::{ErrorKind, FieldError};

/// 错误的语义描述契约。
///
/// 各层的错误枚举实现此 trait 后，即可被统一渲染为对外错误表示，无需在传输层
/// 为每个业务模块编写映射代码。
///
/// # 外层错误的实现方式
///
/// 上层错误包裹下层错误时，直接委托即可，不必重复枚举：
///
/// ```
/// # use platform_kernel::error::{ErrorKind, ErrorMeta};
/// # #[derive(Debug, thiserror::Error)]
/// # #[error("domain")]
/// # struct DomainError;
/// # impl ErrorMeta for DomainError {
/// #     fn kind(&self) -> ErrorKind { ErrorKind::NotFound }
/// #     fn code(&self) -> &'static str { "x.y" }
/// # }
/// #[derive(Debug, thiserror::Error)]
/// enum AppError {
///     #[error(transparent)]
///     Domain(#[from] DomainError),
///     #[error("并发冲突")]
///     Conflict,
/// }
///
/// impl ErrorMeta for AppError {
///     fn kind(&self) -> ErrorKind {
///         match self {
///             Self::Domain(e) => e.kind(),
///             Self::Conflict => ErrorKind::Conflict,
///         }
///     }
///
///     fn code(&self) -> &'static str {
///         match self {
///             Self::Domain(e) => e.code(),
///             Self::Conflict => "app.conflict",
///         }
///     }
/// }
/// ```
pub trait ErrorMeta {
    /// 语义类别，决定传输层的状态码映射与日志级别。
    fn kind(&self) -> ErrorKind;

    /// 稳定的机器可读错误码，格式为 `模块.具体错误`，全小写蛇形。
    ///
    /// 这是**对外 API 契约的一部分**：客户端会针对特定 code 编写分支逻辑，
    /// 因此一经发布不得修改，只能新增。
    ///
    /// 返回 `&'static str` 而非 `String` 是刻意为之 —— 强制取值来自有限集合，
    /// 杜绝把用户输入拼进错误码这类污染。
    fn code(&self) -> &'static str;

    /// 面向调用方的补充说明。
    ///
    /// 服务端类错误（[`ErrorKind::is_detail_safe_to_expose`] 为 `false`）返回的
    /// 内容会在传输层被丢弃，因此这里无需自行判断是否脱敏。
    fn detail(&self) -> Option<Cow<'_, str>> {
        None
    }

    /// 字段级校验错误明细，仅 [`ErrorKind::Validation`] 场景需要覆写。
    fn fields(&self) -> Vec<FieldError> {
        Vec::new()
    }

    /// 原样重试是否有意义。
    ///
    /// 该取值同时服务于两个消费者：对外通过 `Retry-After` 等语义告知客户端，
    /// 对内供重试中间件判断是否发起下一次尝试。两者共用同一判断，避免出现
    /// 「客户端在重试而服务端认为不该重试」这类不一致。
    fn retryable(&self) -> bool {
        self.kind().retryable_by_default()
    }
}

impl<T> ErrorMeta for &T
where
    T: ErrorMeta + ?Sized,
{
    fn kind(&self) -> ErrorKind {
        (**self).kind()
    }

    fn code(&self) -> &'static str {
        (**self).code()
    }

    fn detail(&self) -> Option<Cow<'_, str>> {
        (**self).detail()
    }

    fn fields(&self) -> Vec<FieldError> {
        (**self).fields()
    }

    fn retryable(&self) -> bool {
        (**self).retryable()
    }
}

impl<T> ErrorMeta for Box<T>
where
    T: ErrorMeta + ?Sized,
{
    fn kind(&self) -> ErrorKind {
        (**self).kind()
    }

    fn code(&self) -> &'static str {
        (**self).code()
    }

    fn detail(&self) -> Option<Cow<'_, str>> {
        (**self).detail()
    }

    fn fields(&self) -> Vec<FieldError> {
        (**self).fields()
    }

    fn retryable(&self) -> bool {
        (**self).retryable()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, thiserror::Error)]
    #[error("测试错误")]
    struct Sample(ErrorKind);

    impl ErrorMeta for Sample {
        fn kind(&self) -> ErrorKind {
            self.0
        }

        fn code(&self) -> &'static str {
            "sample.err"
        }
    }

    #[test]
    fn caller_fault_excluded_from_server_error_rate() {
        assert!(ErrorKind::Validation.is_caller_fault());
        assert!(ErrorKind::NotFound.is_caller_fault());
        assert!(!ErrorKind::Internal.is_caller_fault());
        assert!(!ErrorKind::Unavailable.is_caller_fault());
    }

    #[test]
    fn internal_detail_never_exposed() {
        assert!(!ErrorKind::Internal.is_detail_safe_to_expose());
        assert!(!ErrorKind::Timeout.is_detail_safe_to_expose());
        assert!(ErrorKind::Validation.is_detail_safe_to_expose());
    }

    #[test]
    fn default_retry_policy_covers_transient_only() {
        assert!(Sample(ErrorKind::Unavailable).retryable());
        assert!(Sample(ErrorKind::Timeout).retryable());
        assert!(!Sample(ErrorKind::Validation).retryable());
        assert!(!Sample(ErrorKind::Internal).retryable());
    }

    #[test]
    fn reference_and_box_delegate_semantics() {
        let e = Sample(ErrorKind::Conflict);
        assert_eq!(ErrorMeta::kind(&&e), ErrorKind::Conflict);

        let boxed: Box<dyn ErrorMeta> = Box::new(Sample(ErrorKind::Forbidden));
        assert_eq!(boxed.kind(), ErrorKind::Forbidden);
        assert_eq!(boxed.code(), "sample.err");
    }

    #[test]
    fn field_error_skips_empty_message() {
        let fe = FieldError::new("email", "required");
        let json = serde_json::to_string(&fe).expect("序列化不应失败");
        assert_eq!(json, r#"{"field":"email","code":"required"}"#);
    }
}
