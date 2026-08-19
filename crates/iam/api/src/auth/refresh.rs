use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use iam_application::{commands::RefreshTokenCommand, error::AppError};

use crate::{
    response::{ApiError, ApiOk},
    state::CommandState,
};

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RefreshTokenReq {
    #[validate(length(min = 1, message = "refresh_token 不能为空"))]
    refresh_token: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RefreshTokenRes {
    pub access_token: String,
    pub refresh_token: String,
    pub access_expires_at: chrono::DateTime<chrono::Utc>,
    pub refresh_expires_at: chrono::DateTime<chrono::Utc>,
}

#[utoipa::path(
    post,
    path = "/refresh",
    request_body = RefreshTokenReq,
    responses(
        (status = 200, description = "刷新成功，返回新的一对令牌"),
        (status = 401, description = "refresh_token 无效或已过期，需要重新登录"),
        (status = 403, description = "账号已被禁用或离职"),
    ),
    tag = "IAM.Auth"
)]
pub async fn refresh_token(
    State(state): State<CommandState>,
    Json(req): Json<RefreshTokenReq>,
) -> Result<ApiOk<RefreshTokenRes>, ApiError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let command = RefreshTokenCommand {
        refresh_token: req.refresh_token,
    };

    let result = iam_application::commands::handle_refresh_token(
        &*state.uow_factory,
        &*state.token_service,
        command,
    )
    .await
    .map_err(AppError::from)?;

    Ok(ApiOk::data(RefreshTokenRes {
        access_token: result.access_token,
        refresh_token: result.refresh_token,
        access_expires_at: result.access_expires_at,
        refresh_expires_at: result.refresh_expires_at,
    }))
}
