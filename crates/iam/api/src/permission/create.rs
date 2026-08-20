use axum::{Json, extract::State};
use platform_security::context::SecurityContext;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use iam_application::error::AppError;

use crate::{
    response::{ApiError, ApiOk},
    state::CommandState,
};

// =========================================================================
// 权限创建 (Create Permission)
// =========================================================================

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreatePermissionReq {
    /// 父级权限ID，不传表示创建根节点
    #[schema(example = "018f3d61-9c12-7bb3-a00d-5a81e9f1a234")]
    parent_id: Option<Uuid>,

    #[validate(length(min = 1, max = 64, message = "权限名称长度必须在 1-64 之间"))]
    #[schema(example = "新增用户")]
    name: String,

    #[validate(length(min = 1, max = 128, message = "权限编码长度必须在 1-128 之间"))]
    #[schema(example = "iam:user:add")]
    code: String,

    /// 权限类型：menu / button / api
    #[validate(length(min = 1, max = 32, message = "权限类型不能为空"))]
    #[schema(example = "api")]
    kind: String,

    /// 前端路由路径（仅 kind = menu 时有意义）
    #[validate(length(max = 255, message = "路由路径长度不能超过 255"))]
    #[schema(example = "/system/user")]
    route_path: Option<String>,

    /// 前端组件路径（仅 kind = menu 时有意义）
    #[validate(length(max = 255, message = "组件路径长度不能超过 255"))]
    #[schema(example = "views/system/user/index")]
    component: Option<String>,

    /// 菜单图标（仅 kind = menu 时有意义）
    #[validate(length(max = 128, message = "图标长度不能超过 128"))]
    #[schema(example = "user-icon")]
    icon: Option<String>,

    /// 接口请求方法（仅 kind = api 时有意义）：GET / POST / PUT / DELETE / PATCH / HEAD / OPTIONS
    #[schema(example = "POST")]
    api_method: Option<String>,

    /// 接口路径（仅 kind = api 时有意义）
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
pub struct CreatePermissionRes {}

#[utoipa::path(
    post,
    path = "",
    request_body = CreatePermissionReq,
    responses(
        (status = 200, description = "权限创建成功"),
        (status = 400, description = "参数校验失败，或权限类型与附属字段不匹配"),
        (status = 409, description = "权限名称/权限编码 已存在"),
    ),
    tag = "IAM.Permission"
)]
pub async fn create_permission(
    State(state): State<CommandState>,
    ctx: SecurityContext,
    Json(req): Json<CreatePermissionReq>,
) -> Result<ApiOk<CreatePermissionRes>, ApiError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // 组装应用层 Command
    let command = iam_application::commands::PermissionCreateCommand {
        parent_id: req.parent_id,
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
        operator_id: Some(ctx.id()),
    };

    iam_application::commands::handle_permission_create(
        &*state.uow_factory,
        &*state.clock,
        command,
    )
    .await
    .map_err(AppError::from)?;

    Ok(ApiOk::data(CreatePermissionRes {}))
}
