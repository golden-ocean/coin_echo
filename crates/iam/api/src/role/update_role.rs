use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use iam_application::error::AppError;

use crate::{api_error::ApiError, api_res::ApiOk, state::CommandState};

// =========================================================================
// 角色更新 (Update Role)
// =========================================================================
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateReq {
    #[validate(length(min = 2, max = 32, message = "角色名称长度必须在 2-32 之间"))]
    #[schema(example = "管理员")]
    name: String,
    #[validate(length(min = 2, max = 64, message = "角色编码长度必须在 2-64 之间"))]
    #[schema(example = "admin")]
    code: String,
    #[validate(range(min = 0, max = 9999, message = "排序值必须在 0-9999 之间"))]
    #[schema(example = 10)]
    sort: Option<i32>,
    #[validate(length(max = 255, message = "备注长度不能超过 255"))]
    #[schema(example = "平台内置角色")]
    remark: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UpdateRes {}

#[utoipa::path(
    put,
    path = "/roles/{id}",
    params(
        ("id" = Uuid, Path, description = "角色唯一ID", example = "018f3d61-9c12-7bb3-a00d-5a81e9f1a234")
    ),
    request_body = UpdateReq,
    responses(
        (status = 200, description = "角色创建成功"),
        (status = 400, description = "参数校验失败"),
        (status = 404, description = "角色不存在"),
    ),
    tag = "IAM.Role"
)]
pub async fn update_role(
    Path(id): Path<Uuid>,
    State(state): State<CommandState>,
    Json(req): Json<UpdateReq>,
) -> Result<ApiOk<UpdateRes>, ApiError<AppError>> {
    req.validate()
        .map_err(|e| ApiError::iam(AppError::Validation(e.to_string())))?;

    // TODO: 替换为真实的 AuthExtractor 提取当前操作人
    let current_operator_id = Some(Uuid::now_v7());

    let command = iam_application::commands::RoleUpdateCommand {
        id,
        name: req.name,
        code: req.code,
        sort: req.sort,
        remark: req.remark,
        operator_id: current_operator_id,
    };

    iam_application::commands::handle_role_update(&*state.uow_factory, &*state.clock, command)
        .await
        .map_err(ApiError::iam)?;

    Ok(ApiOk::data(UpdateRes {}))
}
