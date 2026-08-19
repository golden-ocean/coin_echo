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
// 权限更新 (Update Permission)
// =========================================================================
// 注意：不包含 parent_id / status。
// - parent_id（树形结构调整）涉及多级循环引用校验，属于独立业务操作，走单独的
//   change-parent 接口，不和基础信息更新混在一起。
// - status 的启用/禁用通过独立的 enable/disable 接口触发（对应领域方法
//   Permission::enable / disable），保持"一个接口只做一件事"。

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdatePermissionReq {
    #[validate(length(min = 1, max = 64, message = "权限名称长度必须在 1-64 之间"))]
    #[schema(example = "新增用户")]
    name: String,

    #[validate(length(min = 1, max = 128, message = "权限编码长度必须在 1-128 之间"))]
    #[schema(example = "iam:user:add")]
    code: String,

    #[validate(length(min = 1, max = 32, message = "权限类型不能为空"))]
    #[schema(example = "api")]
    kind: String,

    #[validate(length(max = 255, message = "路由路径长度不能超过 255"))]
    #[schema(example = "/system/user")]
    route_path: Option<String>,

    #[validate(length(max = 255, message = "组件路径长度不能超过 255"))]
    #[schema(example = "views/system/user/index")]
    component: Option<String>,

    #[validate(length(max = 128, message = "图标长度不能超过 128"))]
    #[schema(example = "user-icon")]
    icon: Option<String>,

    #[schema(example = "POST")]
    api_method: Option<String>,

    #[validate(length(max = 255, message = "接口路径长度不能超过 255"))]
    #[schema(example = "/api/v1/users")]
    api_path: Option<String>,

    #[validate(range(min = 0, max = 9999, message = "排序值必须在 0-9999 之间"))]
    #[schema(example = 10)]
    sort: Option<i32>,

    #[validate(length(max = 255, message = "备注长度不能超过 255"))]
    #[schema(example = "新增用户接口权限")]
    remark: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UpdatePermissionRes {}

#[utoipa::path(
    put,
    path = "/{id}",
    params(
        ("id" = Uuid, Path, description = "权限唯一ID", example = "018f3d61-9c12-7bb3-a00d-5a81e9f1a234")
    ),
    request_body = UpdatePermissionReq,
    responses(
        (status = 200, description = "权限更新成功"),
        (status = 400, description = "参数校验失败，或权限类型与附属字段不匹配"),
        (status = 403, description = "系统内置权限受保护，禁止修改"),
        (status = 404, description = "权限不存在"),
    ),
    tag = "IAM.Permission"
)]
pub async fn update_permission(
    Path(id): Path<Uuid>,
    State(state): State<CommandState>,
    ctx: SecurityContext,
    Json(req): Json<UpdatePermissionReq>,
) -> Result<ApiOk<UpdatePermissionRes>, ApiError<AppError>> {
    state
        .enforcer
        .check(&ctx.id().to_string(), "iam::permission::update")
        .await
        .map_err(|_| ApiError::iam(AppError::Unauthorized))?;

    req.validate()
        .map_err(|e| ApiError::iam(AppError::Validation(e.to_string())))?;

    let current_operator_id = Some(ctx.id());

    let command = iam_application::commands::PermissionUpdateCommand {
        id,
        name: req.name,
        code: req.code,
        kind: req.kind,
        route_path: req.route_path,
        component: req.component,
        icon: req.icon,
        api_method: req.api_method,
        api_path: req.api_path,
        sort: req.sort,
        remark: req.remark,
        operator_id: current_operator_id,
    };

    iam_application::commands::handle_permission_update(
        &*state.uow_factory,
        &*state.clock,
        command,
    )
    .await
    .map_err(ApiError::iam)?;

    Ok(ApiOk::data(UpdatePermissionRes {}))
}
