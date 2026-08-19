use axum::extract::{Path, State};

use platform_security::context::SecurityContext;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use iam_application::error::AppError;

use crate::{
    response::{ApiError, ApiOk},
    state::CommandState,
};

// =========================================================================
// 权限删除 (Delete Permission)
// =========================================================================

#[derive(Debug, Serialize, ToSchema)]
pub struct DeletePermissionRes {}

#[utoipa::path(
    delete,
    path = "/{id}",
    params(
        ("id" = Uuid, Path, description = "权限唯一ID", example = "018f3d61-9c12-7bb3-a00d-5a81e9f1a234")
    ),
    responses(
        (status = 200, description = "权限删除成功"),
        (status = 403, description = "系统内置权限受保护，禁止删除"),
        (status = 404, description = "权限不存在"),
        (status = 409, description = "权限存在子节点，或已被并发修改（版本冲突）"),
    ),
    tag = "IAM.Permission"
)]
pub async fn delete_permission(
    Path(id): Path<Uuid>,
    State(state): State<CommandState>,
    ctx: SecurityContext,
) -> Result<ApiOk<DeletePermissionRes>, ApiError> {
    state
        .enforcer
        .check(&ctx.id().to_string(), "iam::permission::delete")
        .await
        .map_err(|_| AppError::Forbidden)?;

    // 组装应用层 Command
    // 注意：删除前是否存在子节点（has_children）的校验放在应用层 command handler
    // 中完成 —— 仓储层已提供 PermissionRepository::has_children 用于该前置检查，
    // 防止产生孤儿节点。
    let command = iam_application::commands::PermissionDeleteCommand {
        id,
        operator_id: Some(ctx.id()),
    };

    iam_application::commands::handle_permission_delete(
        &*state.uow_factory,
        &*state.clock,
        command,
    )
    .await
    .map_err(AppError::from)?;

    Ok(ApiOk::data(DeletePermissionRes {}))
}
