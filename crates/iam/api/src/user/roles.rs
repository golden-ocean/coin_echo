use axum::extract::{Path, State};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use iam_application::error::AppError;

use crate::{api_error::ApiError, api_res::ApiOk, state::QueryState};

#[derive(Debug, Serialize, ToSchema)]
pub struct UserRoleIdsRes {
    #[schema(example = "[\"018f3d61-9c12-7bb3-a00d-5a81e9f1a234\"]")]
    pub role_ids: Vec<Uuid>,
}

#[utoipa::path(
    get,
    path = "/roles",
    params(
        ("id" = Uuid, Path, description = "用户唯一ID", example = "018f3d61-9c12-7bb3-a00d-5a81e9f1a234")
    ),
    responses(
        (status = 200, description = "用户当前拥有的角色 ID 列表，用于前端角色多选框回显"),
    ),
    tag = "IAM.User"
)]
pub async fn get_user_roles(
    Path(id): Path<Uuid>,
    State(state): State<QueryState>,
) -> Result<ApiOk<UserRoleIdsRes>, ApiError<AppError>> {
    let role_ids = iam_application::queries::handle_user_role_ids(&state.reader_pool, id)
        .await
        .map_err(ApiError::iam)?;

    Ok(ApiOk::data(UserRoleIdsRes { role_ids }))
}
