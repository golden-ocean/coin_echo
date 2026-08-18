use axum::{
    Extension, Json,
    extract::{Path, State},
};
use platform_middleware::CurrentUser;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use iam_application::error::AppError;

use crate::{api_error::ApiError, api_res::ApiOk, state::CommandState};

// =========================================================================
// 用户-角色分配 (Assign User Roles)：全量替换，非增量
// =========================================================================

#[derive(Debug, Deserialize, ToSchema)]
pub struct AssignUserRolesReq {
    /// 该用户应拥有的完整角色 ID 列表（全量替换）
    #[schema(example = "[\"018f3d61-9c12-7bb3-a00d-5a81e9f1a234\"]")]
    role_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AssignUserRolesRes {}

#[utoipa::path(
    put,
    path = "/roles",
    params(
        ("id" = Uuid, Path, description = "用户唯一ID", example = "018f3d61-9c12-7bb3-a00d-5a81e9f1a234")
    ),
    request_body = AssignUserRolesReq,
    responses(
        (status = 200, description = "用户角色分配成功（全量替换）"),
        (status = 404, description = "用户不存在，或角色 ID 列表中包含不存在的角色"),
    ),
    tag = "IAM.User"
)]
pub async fn assign_user_roles(
    Path(id): Path<Uuid>,
    State(state): State<CommandState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(req): Json<AssignUserRolesReq>,
) -> Result<ApiOk<AssignUserRolesRes>, ApiError<AppError>> {
    let current_operator_id = Some(current_user.id());

    let command = iam_application::commands::UserAssignRolesCommand {
        user_id: id,
        role_ids: req.role_ids,
        operator_id: current_operator_id,
    };

    iam_application::commands::handle_user_assign_roles(&*state.uow_factory, command)
        .await
        .map_err(ApiError::iam)?;

    Ok(ApiOk::data(AssignUserRolesRes {}))
}
