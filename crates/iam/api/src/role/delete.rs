use axum::{
    Extension,
    extract::{Path, State},
};
use iam_application::error::AppError;
use platform_middleware::CurrentUser;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{api_error::ApiError, api_res::ApiOk, state::CommandState};

// =========================================================================
// 角色删除 (Delete Role)
// =========================================================================
#[derive(Debug, Serialize, ToSchema)]
pub struct DeleteRoleRes {}

#[utoipa::path(
    delete,
    path = "/{id}",
    params(
        ("id" = Uuid, Path, description = "角色唯一ID", example = "018f3d61-9c12-7bb3-a00d-5a81e9f1a234")
    ),
    responses(
        (status = 200, description = "角色删除成功"),
        (status = 403, description = "系统内置角色受保护，禁止删除"),
        (status = 404, description = "角色不存在"),
        (status = 409, description = "角色已被并发修改，版本冲突"),
    ),
    tag = "IAM.Role"
)]
pub async fn delete_role(
    Path(id): Path<Uuid>,
    State(state): State<CommandState>,
    Extension(current_user): Extension<CurrentUser>,
) -> Result<ApiOk<DeleteRoleRes>, ApiError<AppError>> {
    let current_operator_id = Some(current_user.id());

    let command = iam_application::commands::RoleDeleteCommand {
        id,
        operator_id: current_operator_id,
    };

    iam_application::commands::handle_role_delete(&*state.uow_factory, &*state.clock, command)
        .await
        .map_err(ApiError::iam)?;

    Ok(ApiOk::data(DeleteRoleRes {}))
}
