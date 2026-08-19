use axum::response::IntoResponse;
use iam_application::error::AppError;

use platform_web::ApiError as PlatformWebApiError;

#[derive(Debug)]
pub struct ApiError(PlatformWebApiError<AppError>);

impl From<AppError> for ApiError {
    fn from(err: AppError) -> Self {
        Self(PlatformWebApiError::new(err, "iam"))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        self.0.into_response()
    }
}

pub use platform_web::ApiOk;

// /// iam-api 专属 HTTP 响应类型别名
// pub type ApiResult<T = ()> = Result<ApiOk<T>, ApiError>;
