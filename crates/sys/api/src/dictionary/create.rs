use axum::{Json, extract::State};
use platform_security::context::SecurityContext;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use sys_application::error::AppError;

use crate::{
    response::{ApiError, ApiOk},
    state::CommandState,
};

// =========================================================================
// 字典创建 (Create Dictionary)
// =========================================================================
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateDictionaryReq {
    #[validate(length(min = 2, max = 64, message = "字典名称长度必须在 2-64 之间"))]
    #[schema(example = "性别")]
    name: String,
    #[validate(length(min = 2, max = 64, message = "字典编码长度必须在 2-64 之间"))]
    #[schema(example = "gender")]
    code: String,
    #[validate(range(min = 0, max = 9999, message = "排序值必须在 0-9999 之间"))]
    #[schema(example = 10)]
    sort: Option<i32>,
    #[validate(length(max = 500, message = "备注长度不能超过 500"))]
    #[schema(example = "性别枚举字典")]
    remark: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateDictionaryRes {}

#[utoipa::path(
    post,
    path = "",
    request_body = CreateDictionaryReq,
    responses(
        (status = 200, description = "字典创建成功"),
        (status = 400, description = "参数校验失败"),
        (status = 409, description = "字典名称/编码 已存在"),
    ),
    tag = "SYS.Dictionary"
)]
pub async fn create_dictionary(
    State(state): State<CommandState>,
    ctx: SecurityContext,
    Json(req): Json<CreateDictionaryReq>,
) -> Result<ApiOk<CreateDictionaryRes>, ApiError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let command = sys_application::commands::DictionaryCreateCommand {
        name: req.name,
        code: req.code,
        sort: req.sort,
        remark: req.remark,
        operator_id: Some(ctx.id()),
    };

    sys_application::commands::handle_dictionary_create(
        &*state.uow_factory,
        &*state.clock,
        command,
    )
    .await
    .map_err(AppError::from)?;

    Ok(ApiOk::data(CreateDictionaryRes {}))
}
