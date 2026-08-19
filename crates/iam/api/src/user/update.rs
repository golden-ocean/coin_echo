use axum::{
    Json,
    extract::{Path, State},
};
use platform_security::context::SecurityContext;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use iam_application::error::AppError;

use crate::{api_error::ApiError, api_res::ApiOk, state::CommandState};

// =========================================================================
// Update 用户信息更新 (Update User Info)
// =========================================================================
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateUserReq {
    #[validate(length(min = 1, max = 50, message = "姓名长度必须在 1-50 之间"))]
    #[schema(example = "李四")]
    pub name: String,
    #[validate(email(message = "邮箱格式不正确"))]
    #[schema(example = "lisi@example.com")]
    pub email: String,
    /// 手机号，11-15 位
    #[validate(length(min = 11, max = 15, message = "手机号长度必须在 11-15 之间"))]
    #[schema(example = "13912345678")]
    pub phone: String,
    #[schema(example = "018f3d61-9c12-7bb3-a00d-5a81e9f1a234")]
    pub organization_id: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UpdateUserRes {}

#[utoipa::path(
    put,
    path = "/{id}",
    params(
        ("id" = Uuid, Path, description = "用户ID", example = "018f3d61-9c12-7bb3-a00d-5a81e9f1a234")
    ),
    request_body = UpdateUserReq,
    responses(
        (status = 200, description = "用户信息更新成功"),
        (status = 400, description = "参数校验失败"),
        (status = 404, description = "用户不存在或已删除"),
        (status = 409, description = "邮箱/手机号已存在，或版本冲突"),
    ),
    tag = "IAM.User"
)]
pub async fn update_user(
    Path(id): Path<Uuid>,
    State(state): State<CommandState>,
    ctx: SecurityContext,
    Json(req): Json<UpdateUserReq>,
) -> Result<ApiOk<UpdateUserRes>, ApiError<AppError>> {
    req.validate()
        .map_err(|e| ApiError::iam(AppError::Validation(e.to_string())))?;

    state
        .enforcer
        .check(&ctx.id().to_string(), "iam::user::update")
        .await
        .map_err(|_| ApiError::iam(AppError::Unauthorized))?;

    let command = iam_application::commands::UserUpdateCommand {
        id,
        name: req.name,
        email: req.email,
        phone: req.phone,
        organization_id: req.organization_id,
        operator_id: Some(ctx.id()),
    };

    iam_application::commands::handle_user_update(&*state.uow_factory, &*state.clock, command)
        .await
        .map_err(ApiError::iam)?;

    Ok(ApiOk::data(UpdateUserRes {}))
}
