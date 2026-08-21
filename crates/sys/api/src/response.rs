use axum::response::IntoResponse;

use sys_application::error::AppError;

use platform_web::PlatformWebError;

#[derive(Debug)]
pub struct ApiError(PlatformWebError<AppError>);

impl From<AppError> for ApiError {
    fn from(err: AppError) -> Self {
        Self(PlatformWebError::new(err, "sys"))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        self.0.into_response()
    }
}

pub use platform_web::PlatformWebOk as ApiOk;
