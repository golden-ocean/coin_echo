use axum::Json;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use std::borrow::Cow;

use iam_application::error::AppError;
use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};
use platform_kernel::http::ProblemDetails;
use platform_middleware::RequestContext;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error(transparent)]
    App(#[from] AppError),
}

impl ErrorMeta for ApiError {
    fn kind(&self) -> ErrorKind {
        match self {
            Self::App(e) => e.kind(),
        }
    }
    fn code(&self) -> &'static str {
        match self {
            Self::App(e) => e.code(),
        }
    }
    fn detail(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::App(e) => e.detail(),
        }
    }
    fn fields(&self) -> Vec<FieldError> {
        match self {
            Self::App(e) => e.fields(),
        }
    }
    fn retryable(&self) -> bool {
        match self {
            Self::App(e) => e.retryable(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // ✅ task_local 安全获取，未安装时自动降级为空字符串
        let ctx = RequestContext::current_or_default();

        // ✅ 正确解构 + namespace 绑定
        let (err_ref, namespace) = match &self {
            ApiError::App(_) => (&self as &dyn ErrorMeta, "iam"),
        };

        let problem = ProblemDetails::from_error(err_ref, namespace, ctx.instance, ctx.trace_id);

        let status =
            StatusCode::from_u16(problem.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        let mut response = Json(problem).into_response();
        *response.status_mut() = status;

        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        );

        // ✅ 可重试时附加 Retry-After
        if err_ref.retryable() {
            let secs = match err_ref.kind() {
                ErrorKind::Exhausted => "5",
                ErrorKind::Unavailable => "10",
                ErrorKind::Timeout => "3",
                _ => "1",
            };
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from_static(secs));
        }

        response
    }
}
