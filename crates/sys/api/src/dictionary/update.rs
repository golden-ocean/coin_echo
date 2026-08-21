use axum::{
    Json,
    extract::{Path, State},
};
use platform_security::context::SecurityContext;
use serde::{Deserialize, Serialize};
use sys_application::error::AppError;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::{
    response::{ApiError, ApiOk},
    state::CommandState,
};

// =========================================================================
// 字典信息更新 (Update Dictionary)
// =========================================================================
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateDictionaryReq {
    #[validate(length(min = 2, max = 64, message = "字典名称长度必须在 2-64 之间"))]
    #[schema(example = "性别")]
    name: String,
    #[validate(length(max = 500, message = "备注长度不能超过 500"))]
    #[schema(example = "性别枚举字典")]
    remark: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UpdateDictionaryRes {}

#[utoipa::path(
    put,
    path = "/{id}",
    request_body = UpdateDictionaryReq,
    params(("id" = Uuid, Path, description = "字典ID", example = "018f3d61-9c12-7bb3-a00d-5a81e9f1a234")),
    responses(
        (status = 200, description = "字典信息更新成功"),
        (status = 400, description = "参数校验失败"),
        (status = 403, description = "内置字典不可修改"),
        (status = 404, description = "字典不存在"),
        (status = 409, description = "字典名称已存在"),
    ),
    tag = "SYS.Dictionary"
)]
pub async fn update_dictionary(
    State(state): State<CommandState>,
    ctx: SecurityContext,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateDictionaryReq>,
) -> Result<ApiOk<UpdateDictionaryRes>, ApiError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let command = sys_application::commands::DictionaryUpdateCommand {
        id,
        name: req.name,
        remark: req.remark,
        operator_id: Some(ctx.id()),
    };

    sys_application::commands::handle_dictionary_update(
        &*state.uow_factory,
        &*state.clock,
        command,
    )
    .await
    .map_err(AppError::from)?;

    Ok(ApiOk::data(UpdateDictionaryRes {}))
}
