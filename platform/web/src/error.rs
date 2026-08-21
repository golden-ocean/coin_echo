use std::fmt;

use axum::Json;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};

use platform_kernel::error::{ErrorKind, ErrorMeta};
use platform_kernel::http::ProblemDetails;
use platform_middleware::RequestContext;

/// 把业务错误包装成可直接从 handler 返回的类型。
#[derive(Debug)]
pub struct PlatformWebError<E> {
    error: E,
    namespace: &'static str,
}

impl<E: ErrorMeta> PlatformWebError<E> {
    pub fn new(error: E, namespace: &'static str) -> Self {
        Self { error, namespace }
    }

    /// 获取内部错误的引用（供日志/中间件等外部消费者使用）。
    pub fn inner(&self) -> &E {
        &self.error
    }

    pub fn status_code(&self) -> StatusCode {
        match self.error.kind() {
            ErrorKind::Validation => StatusCode::BAD_REQUEST,
            ErrorKind::Unauthenticated => StatusCode::UNAUTHORIZED,
            ErrorKind::Forbidden => StatusCode::FORBIDDEN,
            ErrorKind::NotFound => StatusCode::BAD_REQUEST,
            ErrorKind::Conflict => StatusCode::CONFLICT,
            ErrorKind::Exhausted => StatusCode::TOO_MANY_REQUESTS,
            ErrorKind::Timeout => StatusCode::GATEWAY_TIMEOUT,
            ErrorKind::Unavailable => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorKind::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// 根据错误语义给出建议的 `Retry-After` 秒数。
    ///
    /// 具体等待时长是传输层约定，不进入 kernel：kernel 只声明"是否可重试"，
    /// 各 API 层可以有自己的策略。
    fn retry_after_secs(&self) -> &'static str {
        match self.error.kind() {
            ErrorKind::Exhausted => "5",    // 限流/配额：等待一个保守窗口
            ErrorKind::Unavailable => "10", // 依赖不可用：留出恢复时间
            ErrorKind::Timeout => "3",      // 超时：短退避
            _ => "1",                       // 自定义可重试错误的兜底
        }
    }
}

impl<E: ErrorMeta> fmt::Display for PlatformWebError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.namespace, self.error.code())
    }
}

impl<E: ErrorMeta + fmt::Debug> std::error::Error for PlatformWebError<E> {}

impl<E: ErrorMeta> IntoResponse for PlatformWebError<E> {
    fn into_response(self) -> Response {
        let ctx = RequestContext::current_or_default();
        let problem =
            ProblemDetails::from_error(&self.error, self.namespace, ctx.instance, ctx.trace_id);

        let status =
            StatusCode::from_u16(problem.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        let mut response = Json(problem).into_response();
        *response.status_mut() = status;
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        );

        if self.error.retryable() {
            response.headers_mut().insert(
                header::RETRY_AFTER,
                HeaderValue::from_static(self.retry_after_secs()),
            );
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_middleware::RequestContext;

    #[derive(Debug, thiserror::Error)]
    #[error("测试错误")]
    struct SampleError {
        kind: ErrorKind,
        code: &'static str,
        retryable: bool,
    }

    impl ErrorMeta for SampleError {
        fn kind(&self) -> ErrorKind {
            self.kind
        }
        fn code(&self) -> &'static str {
            self.code
        }
        fn retryable(&self) -> bool {
            self.retryable
        }
    }

    fn sample_not_found() -> SampleError {
        SampleError {
            kind: ErrorKind::NotFound,
            code: "sample.not_found",
            retryable: false,
        }
    }

    fn sample_unavailable() -> SampleError {
        SampleError {
            kind: ErrorKind::Unavailable,
            code: "sample.db_down",
            retryable: true,
        }
    }

    #[test]
    fn maps_to_correct_status_and_content_type() {
        let response = PlatformWebError::new(sample_not_found(), "iam").into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/problem+json"
        );
    }

    #[test]
    fn retryable_error_includes_retry_after_header() {
        let response = PlatformWebError::new(sample_unavailable(), "iam").into_response();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(response.headers().get(header::RETRY_AFTER).unwrap(), "10");
    }

    #[test]
    fn non_retryable_error_has_no_retry_after_header() {
        let response = PlatformWebError::new(sample_not_found(), "iam").into_response();
        assert!(response.headers().get(header::RETRY_AFTER).is_none());
    }

    #[tokio::test]
    async fn injects_trace_id_and_instance_from_task_local() {
        let ctx = RequestContext {
            instance: "/v1/users/42".into(),
            trace_id: "trace-test-001".into(),
        };

        let body = RequestContext::scope(ctx, async {
            let response = PlatformWebError::new(sample_not_found(), "iam").into_response();
            let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()
        })
        .await;

        assert_eq!(body["instance"], "/v1/users/42");
        assert_eq!(body["trace_id"], "trace-test-001");
        assert_eq!(body["code"], "sample.not_found");
        assert_eq!(body["type"], "urn:iam:error:sample.not_found");
    }

    #[test]
    fn degrades_gracefully_without_task_local_context() {
        let response = PlatformWebError::new(sample_not_found(), "iam").into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn implements_std_error_trait() {
        let err = PlatformWebError::new(sample_not_found(), "iam");
        let _: &dyn std::error::Error = &err;
        assert!(format!("{}", err).contains("iam"));
    }
}
