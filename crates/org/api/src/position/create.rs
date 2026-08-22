use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use platform_security::context::SecurityContext;

use org_application::error::AppError;

use crate::{
    response::{ApiError, ApiOk},
    state::CommandState,
};

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreatePositionReq {
    #[validate(length(min = 1, max = 64))]
    #[schema(example = "经理")]
    name: String,
    #[validate(length(min = 1, max = 64))]
    #[schema(example = "manager")]
    code: String,
    #[validate(range(min = 0, max = 9999))]
    sort: Option<i32>,
    #[validate(length(max = 500))]
    remark: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreatePositionRes {}

#[utoipa::path(
    post,
    path = "",
    request_body = CreatePositionReq,
    responses(
        (status = 200, description = "职位创建成功"),
        (status = 400, description = "参数校验失败"),
        (status = 409, description = "职位名称/编码 已存在"),
    ),
    tag = "ORG.Position"
)]
pub async fn create_position(
    State(state): State<CommandState>,
    ctx: SecurityContext,
    Json(req): Json<CreatePositionReq>,
) -> Result<ApiOk<CreatePositionRes>, ApiError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let command = org_application::commands::PositionCreateCommand {
        name: req.name,
        code: req.code,
        sort: req.sort,
        remark: req.remark,
        operator_id: Some(ctx.id()),
    };

    org_application::commands::handle_position_create(&*state.uow_factory, &*state.clock, command)
        .await
        .map_err(AppError::from)?;

    Ok(ApiOk::data(CreatePositionRes {}))
}
