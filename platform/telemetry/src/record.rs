//! 统一的错误观测记录：把任意 `impl ErrorMeta` 转换成结构化日志字段。
//!
//! # 为什么要有这一层
//!
//! 在没有这层之前，`catch_panic`/`rate_limit`/`auth` 等中间件各自手写
//! `tracing::warn!(code = .., kind = .., ...)`，字段名、要不要带 detail、
//! 敏感信息要不要过滤，全靠各处手写时保持一致——容易漂移。
//!
//! [`ErrorObservation`] 把"一个错误该被记录成什么样"的规则集中到一处：
//! - `detail` 只在 [`ErrorKind::is_detail_safe_to_expose`] 为真时才记录，
//!   与 `ProblemDetails::from_error` 对外响应体的脱敏规则保持一致
//!   （日志和响应体的脱敏口径不该出现两套标准）。
//! - 字段集合固定为 `code`/`kind`/`retryable`/`caller_fault`/`detail`/
//!   `field_count`，这个集合本身就是未来 OTel span attributes 的候选
//!   字段——[`record_error`] 内部目前只是发一条 tracing event，等真正
//!   接入 `tracing-opentelemetry` 后，同一份字段会被自动转换为当前 span
//!   的 attributes，不需要调用点做任何改动。

use platform_kernel::error::ErrorMeta;

/// 一次错误观测的结构化快照。
#[derive(Debug, Clone)]
pub struct ErrorObservation {
    pub code: &'static str,
    pub kind: &'static str,
    pub retryable: bool,
    pub caller_fault: bool,
    pub detail: Option<String>,
    pub field_count: usize,
}

impl ErrorObservation {
    /// 从任意 `impl ErrorMeta` 构造观测快照。
    #[must_use]
    pub fn from_error(err: &dyn ErrorMeta) -> Self {
        let kind = err.kind();
        Self {
            code: err.code(),
            kind: kind.as_str(),
            retryable: err.retryable(),
            caller_fault: kind.is_caller_fault(),
            // 与 ProblemDetails::from_error 相同的脱敏口径：服务端错误的
            // detail 不写进日志字段本身可读的结构化数据里，避免内部实现
            // 细节通过日志采集系统扩散到比响应体更宽的受众。
            //
            // 注意：这不代表 detail 完全丢失——caller_fault 为 false 时，
            // 调用方仍可以自行在日志里带上错误的 Display 输出（包含
            // thiserror 的 #[error("...")] 文案），那部分不受这里的脱敏
            // 规则约束，因为那是"给运维看的错误描述"而非"回给客户端的
            // detail 字段"，两者定位不同。
            detail: kind
                .is_detail_safe_to_expose()
                .then(|| err.detail())
                .flatten(),
            field_count: err.fields().len(),
        }
    }
}

/// 记录一次错误观测。目标日志 target 固定为 `"observability::error"`，
/// 便于日后按 target 单独配置采样/过滤规则，或作为迁移到 OTel span
/// 时的识别标记。
pub fn record_error(err: &dyn ErrorMeta) {
    let obs = ErrorObservation::from_error(err);
    tracing::event!(
        target: "observability::error",
        tracing::Level::WARN,
        code = obs.code,
        kind = obs.kind,
        retryable = obs.retryable,
        caller_fault = obs.caller_fault,
        detail = obs.detail.as_deref(),
        field_count = obs.field_count,
        "错误观测",
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_kernel::error::{ErrorKind, FieldError};

    struct SampleError {
        kind: ErrorKind,
        detail: Option<String>,
        fields: Vec<FieldError>,
    }

    impl ErrorMeta for SampleError {
        fn kind(&self) -> ErrorKind {
            self.kind
        }
        fn code(&self) -> &'static str {
            "sample.error"
        }
        fn detail(&self) -> Option<String> {
            self.detail.clone()
        }
        fn fields(&self) -> Vec<FieldError> {
            self.fields.clone()
        }
    }

    #[test]
    fn observation_exposes_detail_for_caller_fault_errors() {
        let err = SampleError {
            kind: ErrorKind::Validation,
            detail: Some("字段格式不正确".to_string()),
            fields: vec![],
        };
        let obs = ErrorObservation::from_error(&err);
        assert_eq!(obs.detail, Some("字段格式不正确".to_string()));
        assert!(obs.caller_fault);
    }

    #[test]
    fn observation_hides_detail_for_server_fault_errors_even_if_present() {
        // 与 ProblemDetails::from_error 相同的脱敏规则：Internal 类错误
        // 即使 detail() 返回了内容，也必须被丢弃，避免内部实现细节
        // （如数据库连接地址）出现在结构化日志字段里。
        let err = SampleError {
            kind: ErrorKind::Internal,
            detail: Some("db connection to 10.0.1.5 failed".to_string()),
            fields: vec![],
        };
        let obs = ErrorObservation::from_error(&err);
        assert_eq!(obs.detail, None);
        assert!(!obs.caller_fault);
    }

    #[test]
    fn observation_carries_code_kind_and_retryable() {
        let err = SampleError {
            kind: ErrorKind::Unavailable,
            detail: None,
            fields: vec![],
        };
        let obs = ErrorObservation::from_error(&err);
        assert_eq!(obs.code, "sample.error");
        assert_eq!(obs.kind, "unavailable");
        assert!(obs.retryable);
    }

    #[test]
    fn observation_counts_validation_fields() {
        let err = SampleError {
            kind: ErrorKind::Validation,
            detail: None,
            fields: vec![FieldError::new("email", "required")],
        };
        let obs = ErrorObservation::from_error(&err);
        assert_eq!(obs.field_count, 1);
    }

    #[test]
    fn record_error_does_not_panic() {
        // record_error 本身没有可断言的返回值（发一条 tracing event），
        // 这里只保证调用路径本身不会因为字段类型不匹配等问题在运行时
        // panic——真正的日志内容需要外部 subscriber 才能观察，属于
        // init.rs 集成测试的范畴，不在这里覆盖。
        let err = SampleError {
            kind: ErrorKind::Conflict,
            detail: Some("并发冲突".to_string()),
            fields: vec![],
        };
        record_error(&err);
    }
}
