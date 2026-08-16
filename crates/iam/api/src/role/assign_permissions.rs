use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use iam_application::error::AppError;

use crate::{api_error::ApiError, api_res::ApiOk, state::CommandState};

// =========================================================================
// 角色-权限分配 (Assign Role Permissions)：全量替换，非增量
// =========================================================================

#[derive(Debug, Deserialize, ToSchema)]
pub struct AssignRolePermissionsReq {
    /// 该角色应拥有的完整权限 ID 列表（全量替换：传什么就是什么，未出现的权限会被移除）
    #[schema(example = "[\"018f3d61-9c12-7bb3-a00d-5a81e9f1a234\"]")]
    permission_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AssignRolePermissionsRes {}

#[utoipa::path(
    put,
    path = "/permissions",
    params(
        ("id" = Uuid, Path, description = "角色唯一ID", example = "018f3d61-9c12-7bb3-a00d-5a81e9f1a234")
    ),
    request_body = AssignRolePermissionsReq,
    responses(
        (status = 200, description = "角色权限分配成功（全量替换）"),
        (status = 404, description = "角色不存在，或权限 ID 列表中包含不存在的权限"),
    ),
    tag = "IAM.Role"
)]
pub async fn assign_role_permissions(
    Path(id): Path<Uuid>,
    State(state): State<CommandState>,
    Json(req): Json<AssignRolePermissionsReq>,
) -> Result<ApiOk<AssignRolePermissionsRes>, ApiError<AppError>> {
    // TODO: 替换为真实的 AuthExtractor 提取当前操作人
    let current_operator_id = Some(Uuid::now_v7());

    let command = iam_application::commands::RoleAssignPermissionsCommand {
        role_id: id,
        permission_ids: req.permission_ids,
        operator_id: current_operator_id,
    };

    iam_application::commands::handle_role_assign_permissions(&*state.uow_factory, command)
        .await
        .map_err(ApiError::iam)?;

    Ok(ApiOk::data(AssignRolePermissionsRes {}))
}

