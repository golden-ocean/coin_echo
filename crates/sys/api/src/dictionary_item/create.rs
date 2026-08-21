use axum::{Json, extract::State};
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
// 字典项创建 (Create DictionaryItem)
// =========================================================================
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateDictionaryItemReq {
    #[schema(example = "3c9e1a2e-1234-4a5b-9abc-1234567890ab")]
    dictionary_id: Uuid,
    #[validate(length(min = 1, max = 64, message = "显示名称长度必须在 1-64 之间"))]
    #[schema(example = "男")]
    label: String,
    #[validate(length(min = 1, max = 128, message = "枚举值长度必须在 1-128 之间"))]
    #[schema(example = "male")]
    value: String,
    #[schema(example = "#1890ff")]
    color: Option<String>,
    #[validate(range(min = 0, max = 9999, message = "排序值必须在 0-9999 之间"))]
    #[schema(example = 10)]
    sort: Option<i32>,
    #[validate(length(max = 500, message = "备注长度不能超过 500"))]
    remark: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateDictionaryItemRes {}

#[utoipa::path(
    post,
    path = "",
    request_body = CreateDictionaryItemReq,
    responses(
        (status = 200, description = "字典项创建成功"),
        (status = 400, description = "参数校验失败"),
        (status = 404, description = "所属字典不存在"),
        (status = 409, description = "字典项标签/枚举值 已存在"),
    ),
    tag = "SYS.DictionaryItem"
)]
pub async fn create_dictionary_item(
    State(state): State<CommandState>,
    ctx: SecurityContext,
    Json(req): Json<CreateDictionaryItemReq>,
) -> Result<ApiOk<CreateDictionaryItemRes>, ApiError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let command = sys_application::commands::DictionaryItemCreateCommand {
        dictionary_id: req.dictionary_id,
        label: req.label,
        value: req.value,
        color: req.color,
        sort: req.sort,
        remark: req.remark,
        operator_id: Some(ctx.id()),
    };

    sys_application::commands::handle_dictionary_item_create(
        &*state.uow_factory,
        &*state.clock,
        command,
    )
    .await
    .map_err(AppError::from)?;

    Ok(ApiOk::data(CreateDictionaryItemRes {}))
}
