use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use platform_security::context::SecurityContext;
use sys_application::error::AppError;

use crate::{
    response::{ApiError, ApiOk},
    state::CommandState,
};

// =========================================================================
// 字典项展示信息更新 (Update Display：label/color/remark，内置项也可调用)
// =========================================================================
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateDictionaryItemReq {
    #[validate(length(min = 1, max = 64, message = "显示名称长度必须在 1-64 之间"))]
    #[schema(example = "男")]
    label: String,
    #[schema(example = "#1890ff")]
    color: Option<String>,
    #[validate(length(max = 500, message = "备注长度不能超过 500"))]
    remark: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UpdateDictionaryItemRes {}

#[utoipa::path(
    put,
    path = "/{id}",
    request_body = UpdateDictionaryItemReq,
    params(("id" = Uuid, Path, description = "字典项ID", example = "018f3d61-9c12-7bb3-a00d-5a81e9f1a234")),
    responses(
        (status = 200, description = "字典项更新成功"),
        (status = 400, description = "参数校验失败"),
        (status = 404, description = "字典项不存在"),
        (status = 409, description = "字典项标签已存在"),
    ),
    tag = "SYS.DictionaryItem"
)]
pub async fn update_dictionary_item(
    State(state): State<CommandState>,
    ctx: SecurityContext,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateDictionaryItemReq>,
) -> Result<ApiOk<UpdateDictionaryItemRes>, ApiError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let command = sys_application::commands::DictionaryItemUpdateCommand {
        id,
        label: req.label,
        color: req.color,
        remark: req.remark,
        operator_id: Some(ctx.id()),
    };

    sys_application::commands::handle_dictionary_item_update(
        &*state.uow_factory,
        &*state.clock,
        command,
    )
    .await
    .map_err(AppError::from)?;

    Ok(ApiOk::data(UpdateDictionaryItemRes {}))
}
