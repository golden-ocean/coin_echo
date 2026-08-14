//! RFC 9457 (Problem Details for HTTP APIs) 响应体定义
//! https://www.rfc-editor.org/rfc/rfc9457
//!
//! 字段设计对照表：
//! - type    <- 团队错误码（urn:{namespace}:error:{CODE} 格式，不要求真实可访问，只要求全局唯一稳定）
//! - title   <- ErrorKind 的通用简述（同一分类下固定不变）
//! - status  <- HTTP 状态码（body 内冗余一份，方便无状态行的日志系统/消息队列场景解析）
//! - detail  <- 本次请求的具体描述，5xx 场景下为 None，避免泄漏内部细节
//! - instance<- 触发错误的请求路径，由中间件统一回填
//! - code/trace_id/errors 为团队自定义扩展字段

use std::borrow::Cow;

use crate::error::{ErrorKind, ErrorMeta, FieldError};
use serde::Serialize;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ProblemDetails {
    #[serde(rename = "type")]
    pub type_: String,
    pub title: &'static str,
    pub status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// 触发错误的请求路径，由中间件统一回填。
    pub instance: String,
    /// 内部错误码全串，即 `ErrorMeta::code()` 的原样输出。
    pub code: &'static str,
    /// 链路追踪 ID，由中间件统一回填。
    pub trace_id: String,
    /// 请求体字段级校验错误明细，仅在存在字段错误时有值。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<FieldError>>,
}

/// `ErrorKind` → HTTP 状态码。唯一映射入口，杜绝各 handler 各写一套。
fn status_of(kind: ErrorKind) -> u16 {
    match kind {
        ErrorKind::Validation => 400,
        ErrorKind::Unauthenticated => 401,
        ErrorKind::Forbidden => 403,
        ErrorKind::NotFound => 404,
        ErrorKind::Conflict => 409,
        ErrorKind::Exhausted => 429,
        ErrorKind::Timeout => 504, // 上游超时，网关语义
        ErrorKind::Unavailable => 503,
        ErrorKind::Internal => 500,
    }
}

/// `ErrorKind` → RFC 9457 `title`。同一分类下固定不变。
fn title_of(kind: ErrorKind) -> &'static str {
    match kind {
        ErrorKind::Validation => "Validation Failed",
        ErrorKind::Unauthenticated => "Unauthenticated",
        ErrorKind::Forbidden => "Forbidden",
        ErrorKind::NotFound => "Not Found",
        ErrorKind::Conflict => "Conflict",
        ErrorKind::Exhausted => "Too Many Requests",
        ErrorKind::Timeout => "Gateway Timeout",
        ErrorKind::Unavailable => "Service Unavailable",
        ErrorKind::Internal => "Internal Server Error",
    }
}

impl ProblemDetails {
    /// 唯一构造入口。`namespace` 由调用方（各领域 API 层）传入，kernel 本身
    /// 不写死任何业务前缀；`instance`/`trace_id` 由最外层中间件统一取值后传入。
    pub fn from_error(
        err: &dyn ErrorMeta,
        namespace: &str,
        instance: String,
        trace_id: String,
    ) -> Self {
        let kind = err.kind();
        let fields = err.fields();
        Self {
            type_: format!("urn:{namespace}:error:{}", err.code()),
            title: title_of(kind),
            status: status_of(kind),
            detail: if kind.is_detail_safe_to_expose() {
                err.detail().map(Cow::into_owned)
            } else {
                None
            },
            instance,
            code: err.code(),
            trace_id,
            errors: (!fields.is_empty()).then_some(fields),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试用错误：kind/code/detail/fields 均可控，覆盖各分支组合。
    #[derive(Debug, thiserror::Error)]
    #[error("测试错误")]
    struct Sample {
        kind: ErrorKind,
        code: &'static str,
        detail: Option<String>,
        fields: Vec<FieldError>,
    }

    impl Sample {
        fn new(kind: ErrorKind) -> Self {
            Self {
                kind,
                code: "sample.err",
                detail: None,
                fields: Vec::new(),
            }
        }

        fn with_detail(mut self, detail: impl Into<String>) -> Self {
            self.detail = Some(detail.into());
            self
        }

        fn with_fields(mut self, fields: Vec<FieldError>) -> Self {
            self.fields = fields;
            self
        }
    }

    impl ErrorMeta for Sample {
        fn kind(&self) -> ErrorKind {
            self.kind
        }

        fn code(&self) -> &'static str {
            self.code
        }

        fn detail(&self) -> Option<Cow<'_, str>> {
            self.detail.as_deref().map(Cow::Borrowed)
        }

        fn fields(&self) -> Vec<FieldError> {
            self.fields.clone()
        }
    }

    // ---- status_of / title_of：逐一覆盖，防止新增 ErrorKind 分支时漏改 ----

    #[test]
    fn status_of_covers_all_kinds() {
        assert_eq!(status_of(ErrorKind::Validation), 400);
        assert_eq!(status_of(ErrorKind::Unauthenticated), 401);
        assert_eq!(status_of(ErrorKind::Forbidden), 403);
        assert_eq!(status_of(ErrorKind::NotFound), 404);
        assert_eq!(status_of(ErrorKind::Conflict), 409);
        assert_eq!(status_of(ErrorKind::Exhausted), 429);
        assert_eq!(status_of(ErrorKind::Timeout), 504);
        assert_eq!(status_of(ErrorKind::Unavailable), 503);
        assert_eq!(status_of(ErrorKind::Internal), 500);
    }

    #[test]
    fn title_of_covers_all_kinds() {
        assert_eq!(title_of(ErrorKind::Validation), "Validation Failed");
        assert_eq!(title_of(ErrorKind::Unauthenticated), "Unauthenticated");
        assert_eq!(title_of(ErrorKind::Forbidden), "Forbidden");
        assert_eq!(title_of(ErrorKind::NotFound), "Not Found");
        assert_eq!(title_of(ErrorKind::Conflict), "Conflict");
        assert_eq!(title_of(ErrorKind::Exhausted), "Too Many Requests");
        assert_eq!(title_of(ErrorKind::Timeout), "Gateway Timeout");
        assert_eq!(title_of(ErrorKind::Unavailable), "Service Unavailable");
        assert_eq!(title_of(ErrorKind::Internal), "Internal Server Error");
    }

    // ---- from_error：type_/title/status 组装 ----

    #[test]
    fn from_error_builds_urn_type_with_namespace_and_code() {
        let err = Sample::new(ErrorKind::NotFound);
        let pd = ProblemDetails::from_error(&err, "iam", "/v1/users/1".into(), "trace-1".into());
        assert_eq!(pd.type_, "urn:iam:error:sample.err");
    }

    #[test]
    fn from_error_uses_different_namespace_per_call() {
        let err = Sample::new(ErrorKind::NotFound);
        let pd = ProblemDetails::from_error(&err, "task", "/v1/tasks/1".into(), "trace-1".into());
        assert_eq!(pd.type_, "urn:task:error:sample.err");
    }

    #[test]
    fn from_error_sets_title_and_status_from_kind() {
        let err = Sample::new(ErrorKind::Conflict);
        let pd = ProblemDetails::from_error(&err, "iam", "/v1/x".into(), "trace-1".into());
        assert_eq!(pd.status, 409);
        assert_eq!(pd.title, "Conflict");
    }

    #[test]
    fn from_error_copies_code_verbatim() {
        let err = Sample::new(ErrorKind::Validation);
        let pd = ProblemDetails::from_error(&err, "iam", "/v1/x".into(), "trace-1".into());
        assert_eq!(pd.code, "sample.err");
    }

    // ---- from_error：instance/trace_id 原样透传 ----

    #[test]
    fn from_error_passes_instance_and_trace_id_through_unchanged() {
        let err = Sample::new(ErrorKind::Internal);
        let pd = ProblemDetails::from_error(
            &err,
            "iam",
            "/v1/users/42".to_string(),
            "01930a-abcdef".to_string(),
        );
        assert_eq!(pd.instance, "/v1/users/42");
        assert_eq!(pd.trace_id, "01930a-abcdef");
    }

    // ---- from_error：detail 的暴露/脱敏 ----

    #[test]
    fn from_error_exposes_detail_for_caller_fault_kind() {
        // Validation 属于 caller fault，detail 应该透出
        let err = Sample::new(ErrorKind::Validation).with_detail("字段 email 格式不正确");
        let pd = ProblemDetails::from_error(&err, "iam", "/v1/x".into(), "trace-1".into());
        assert_eq!(pd.detail, Some("字段 email 格式不正确".to_string()));
    }

    #[test]
    fn from_error_hides_detail_for_server_fault_kind_even_if_present() {
        // Internal 不是 caller fault，即使 detail() 返回了内容，也必须被丢弃，
        // 防止 SQL 片段/内网地址等实现细节泄漏给客户端。
        let err = Sample::new(ErrorKind::Internal).with_detail("db connection to 10.0.1.5 failed");
        let pd = ProblemDetails::from_error(&err, "iam", "/v1/x".into(), "trace-1".into());
        assert_eq!(pd.detail, None);
    }

    #[test]
    fn from_error_detail_none_when_kind_safe_but_source_has_none() {
        // Validation 允许暴露，但源错误本身没给 detail，结果应仍是 None。
        let err = Sample::new(ErrorKind::Validation);
        let pd = ProblemDetails::from_error(&err, "iam", "/v1/x".into(), "trace-1".into());
        assert_eq!(pd.detail, None);
    }

    // ---- from_error：字段级校验错误 ----

    #[test]
    fn from_error_errors_none_when_no_fields() {
        let err = Sample::new(ErrorKind::Validation);
        let pd = ProblemDetails::from_error(&err, "iam", "/v1/x".into(), "trace-1".into());
        assert!(pd.errors.is_none());
    }

    #[test]
    fn from_error_errors_populated_when_fields_present() {
        let err = Sample::new(ErrorKind::Validation)
            .with_fields(vec![FieldError::new("email", "required")]);
        let pd = ProblemDetails::from_error(&err, "iam", "/v1/x".into(), "trace-1".into());
        let fields = pd.errors.expect("应包含字段错误");
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].field, "email");
        assert_eq!(fields[0].code, "required");
    }

    // ---- JSON 序列化：字段名与省略行为 ----

    #[test]
    fn serializes_type_field_as_reserved_keyword_type() {
        let err = Sample::new(ErrorKind::NotFound);
        let pd = ProblemDetails::from_error(&err, "iam", "/v1/x".into(), "trace-1".into());
        let json = serde_json::to_string(&pd).unwrap();
        assert!(json.contains("\"type\":\"urn:iam:error:sample.err\""));
        // 确认没有把 Rust 字段名 type_ 原样序列化出去
        assert!(!json.contains("type_"));
    }

    #[test]
    fn serializes_without_detail_and_errors_when_absent() {
        let err = Sample::new(ErrorKind::Internal);
        let pd = ProblemDetails::from_error(&err, "iam", "/v1/x".into(), "trace-1".into());
        let json = serde_json::to_string(&pd).unwrap();
        assert!(!json.contains("\"detail\""));
        assert!(!json.contains("\"errors\""));
    }

    #[test]
    fn serializes_with_detail_and_errors_when_present() {
        let err = Sample::new(ErrorKind::Validation)
            .with_detail("请检查输入")
            .with_fields(vec![FieldError::new("email", "required")]);
        let pd = ProblemDetails::from_error(&err, "iam", "/v1/x".into(), "trace-1".into());
        let json = serde_json::to_string(&pd).unwrap();
        assert!(json.contains("\"detail\":\"请检查输入\""));
        assert!(json.contains("\"errors\":["));
    }
}
