use axum::extract::{Path, State};
use iam_application::error::AppError;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{api_error::ApiError, api_res::ApiOk, state::CommandState};

// =========================================================================
// 用户删除 (Delete User)
// =========================================================================
#[derive(Debug, Serialize, ToSchema)]
pub struct DeleteRes {}

#[utoipa::path(
    delete,
    path = "",
    params(
        ("id" = Uuid, Path, description = "用户唯一ID", example = "018f3d61-9c12-7bb3-a00d-5a81e9f1a234")
    ),
    responses(
        (status = 200, description = "用户删除成功"),
        (status = 403, description = "系统内置账户受保护，禁止删除"),
        (status = 404, description = "用户不存在"),
        (status = 409, description = "用户已被并发修改，版本冲突"),
    ),
    tag = "IAM.User"
)]
pub async fn delete_user(
    Path(id): Path<Uuid>,
    State(state): State<CommandState>,
) -> Result<ApiOk<()>, ApiError<AppError>> {
    // TODO: 替换为真实的 AuthExtractor 提取当前操作人
    let current_operator_id = Some(Uuid::now_v7());

    let command = iam_application::commands::UserDeleteCommand {
        id,
        operator_id: current_operator_id,
    };

    iam_application::commands::handle_user_delete(&*state.uow_factory, &*state.clock, command)
        .await
        .map_err(ApiError::iam)?;

    // DELETE 无业务数据返回，走统一信封的 empty 形态
    Ok(ApiOk::empty())
}
