use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use iam_application::{commands::LoginCommand, error::AppError};

use crate::{api_error::ApiError, api_res::ApiOk, state::CommandState};

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct LoginReq {
    #[validate(length(min = 1, max = 64, message = "用户名不能为空"))]
    #[schema(example = "admin")]
    username: String,
    #[validate(length(min = 1, max = 128, message = "密码不能为空"))]
    #[schema(example = "your-password")]
    password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoginRes {
    #[schema(example = "018f3d61-9c12-7bb3-a00d-5a81e9f1a234")]
    pub user_id: String,
    #[schema(example = "admin")]
    pub username: String,
    #[schema(example = "管理员")]
    pub name: String,
    #[schema(example = "[\"018f3d61-9c12-7bb3-a00d-5a81e9f1a234\"]")]
    pub role_ids: Vec<String>,
    pub access_token: String,
    pub refresh_token: String,
    pub access_expires_at: chrono::DateTime<chrono::Utc>,
    pub refresh_expires_at: chrono::DateTime<chrono::Utc>,
}

#[utoipa::path(
    post,
    path = "/login",
    request_body = LoginReq,
    responses(
        (status = 200, description = "登录成功"),
        (status = 400, description = "参数校验失败"),
        (status = 401, description = "用户名或密码错误"),
        (status = 403, description = "账号已被禁用或离职"),
    ),
    tag = "IAM.Auth"
)]
pub async fn login(
    State(state): State<CommandState>,
    Json(req): Json<LoginReq>,
) -> Result<ApiOk<LoginRes>, ApiError<AppError>> {
    req.validate()
        .map_err(|e| ApiError::iam(AppError::Validation(e.to_string())))?;

    let command = LoginCommand {
        username: req.username,
        password: req.password,
    };

    let result = iam_application::commands::handle_login(
        &*state.uow_factory,
        &*state.password_hasher,
        &*state.token_service,
        &*state.clock,
        command,
    )
    .await
    .map_err(ApiError::iam)?;

    Ok(ApiOk::data(LoginRes {
        user_id: result.user_id.to_string(),
        username: result.username,
        name: result.name,
        role_ids: result
            .role_ids
            .into_iter()
            .map(|id| id.to_string())
            .collect(),
        access_token: result.access_token,
        refresh_token: result.refresh_token,
        access_expires_at: result.access_expires_at,
        refresh_expires_at: result.refresh_expires_at,
    }))
}
