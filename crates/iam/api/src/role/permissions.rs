use axum::extract::{Path, State};
use platform_security::context::SecurityContext;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use iam_application::error::AppError;

use crate::{api_error::ApiError, api_res::ApiOk, state::QueryState};

#[derive(Debug, Serialize, ToSchema)]
pub struct RolePermissionIdsRes {
    #[schema(example = "[\"018f3d61-9c12-7bb3-a00d-5a81e9f1a234\"]")]
    pub permission_ids: Vec<Uuid>,
}

#[utoipa::path(
    get,
    path = "/permissions",
    params(
        ("id" = Uuid, Path, description = "角色唯一ID", example = "018f3d61-9c12-7bb3-a00d-5a81e9f1a234")
    ),
    responses(
        (status = 200, description = "角色当前拥有的权限 ID 列表，用于前端权限树勾选回显"),
    ),
    tag = "IAM.Role"
)]
pub async fn list_role_permissions(
    Path(id): Path<Uuid>,
    State(state): State<QueryState>,
    ctx: SecurityContext,
) -> Result<ApiOk<RolePermissionIdsRes>, ApiError<AppError>> {
    state
        .enforcer
        .check(&ctx.id().to_string(), "iam::role::permissions")
        .await
        .map_err(|_| ApiError::iam(AppError::Unauthorized))?;

    let permission_ids =
        iam_application::queries::handle_role_permission_ids(&state.reader_pool, id)
            .await
            .map_err(ApiError::iam)?;

    Ok(ApiOk::data(RolePermissionIdsRes { permission_ids }))
}
