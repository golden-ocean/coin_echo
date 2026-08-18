use axum::{Extension, Json, extract::State};
use platform_kernel::meta::Status;
use platform_middleware::CurrentUser;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use iam_application::error::AppError;

use crate::{api_error::ApiError, api_res::ApiOk, state::CommandState};

// =========================================================================
// Create 用户创建 (Create User)
// =========================================================================
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateUserReq {
    #[validate(length(min = 1, max = 50, message = "用户名长度必须在 1-50 之间"))]
    #[schema(example = "new_user")]
    pub username: String,
    #[validate(length(min = 6, max = 100, message = "密码长度必须在 6-100 之间"))]
    #[schema(example = "password123")]
    pub password: String,
    #[validate(length(min = 1, max = 50, message = "姓名长度必须在 1-50 之间"))]
    #[schema(example = "李四")]
    pub name: String,
    #[validate(email(message = "邮箱格式不正确"))]
    #[schema(example = "lisi@example.com")]
    pub email: String,
    /// 手机号，11 位数字
    #[validate(length(min = 11, max = 15, message = "手机号长度必须在 11-15 之间"))]
    #[schema(example = "13912345678")]
    pub phone: String,
    #[schema(example = "018f3d61-9c12-7bb3-a00d-5a81e9f1a234")]
    pub organization_id: Option<Uuid>,
    #[schema(example = 1000)]
    pub sort: Option<i32>,
    /// 用户状态，可选：enabled / disabled，不传默认 enabled
    #[schema(example = "enabled")]
    pub status: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateUserRes {}

#[utoipa::path(
    post,
    path = "",
    request_body = CreateUserReq,
    responses(
        (status = 200, description = "用户创建成功"),
        (status = 400, description = "参数校验失败"),
        (status = 409, description = "用户名/邮箱/手机号已存在"),
    ),
    tag = "IAM.User"
)]
pub async fn create_user(
    State(state): State<CommandState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(req): Json<CreateUserReq>,
) -> Result<ApiOk<CreateUserRes>, ApiError<AppError>> {
    req.validate()
        .map_err(|e| ApiError::iam(AppError::Validation(e.to_string())))?;

    let current_operator_id = Some(current_user.id());

    let status = req
        .status
        .as_deref()
        .map(|s| {
            s.parse::<Status>()
                .map_err(|e| ApiError::iam(AppError::Validation(e.to_string())))
        })
        .transpose()?;

    let command = iam_application::commands::UserCreateCommand {
        username: req.username,
        password: req.password,
        name: req.name,
        email: req.email,
        phone: req.phone,
        organization_id: req.organization_id,
        sort: req.sort,
        status,
        operator_id: current_operator_id,
    };

    iam_application::commands::handle_user_create(
        &*state.uow_factory,
        &*state.password_hasher,
        &*state.staff_no_generator,
        &*state.clock,
        command,
    )
    .await
    .map_err(ApiError::iam)?;

    Ok(ApiOk::data(CreateUserRes {}))
}
