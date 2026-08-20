use axum::{Json, extract::State};
use platform_security::context::SecurityContext;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use iam_application::error::AppError;

use crate::{
    response::{ApiError, ApiOk},
    state::CommandState,
};

// =========================================================================
// 角色创建 (Create Role)
// =========================================================================
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateRoleReq {
    #[validate(length(min = 2, max = 32, message = "角色名称长度必须在 2-32 之间"))]
    #[schema(example = "管理员")]
    name: String,
    #[validate(length(min = 2, max = 64, message = "角色编码长度必须在 2-64 之间"))]
    #[schema(example = "admin")]
    code: String,
    /// 排序值，非负整数
    #[validate(range(min = 0, max = 9999, message = "排序值必须在 0-9999 之间"))]
    #[schema(example = 10)]
    sort: Option<i32>,
    /// 备注，允许为空
    #[validate(length(max = 255, message = "备注长度不能超过 255"))]
    #[schema(example = "平台内置角色")]
    remark: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CreateRoleRes {}

#[utoipa::path(
    post,
    path = "",
    request_body = CreateRoleReq,
    responses(
        (status = 200, description = "角色创建成功"),
        (status = 400, description = "参数校验失败"),
        (status = 409, description = "角色名称/角色编码 已存在"),
    ),
    tag = "IAM.Role"
)]
pub async fn create_role(
    State(state): State<CommandState>,
    ctx: SecurityContext,
    Json(req): Json<CreateRoleReq>,
) -> Result<ApiOk<CreateRoleRes>, ApiError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    //  组装应用层 Command
    let command = iam_application::commands::RoleCreateCommand {
        name: req.name,
        code: req.code,
        sort: req.sort,
        remark: req.remark,
        operator_id: Some(ctx.id()),
    };

    iam_application::commands::handle_role_create(&*state.uow_factory, &*state.clock, command)
        .await
        .map_err(AppError::from)?;

    Ok(ApiOk::data(CreateRoleRes {}))
}
