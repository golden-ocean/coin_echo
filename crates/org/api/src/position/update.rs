use axum::{
    Json,
    extract::{Path, State},
};
use platform_security::context::SecurityContext;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use org_application::error::AppError;

use crate::{
    response::{ApiError, ApiOk},
    state::CommandState,
};

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdatePositionReq {
    #[validate(length(min = 1, max = 64))]
    name: String,
    #[validate(length(min = 1, max = 64))]
    code: String,
    #[validate(length(max = 500))]
    remark: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UpdatePositionRes {}

#[utoipa::path(
    put,
    path = "/{id}",
    request_body = UpdatePositionReq,
    params(("id" = Uuid, Path, description = "职位ID")),
    responses(
        (status = 200, description = "职位信息更新成功"),
        (status = 400, description = "参数校验失败"),
        (status = 404, description = "职位不存在"),
        (status = 409, description = "职位名称/编码 已存在"),
    ),
    tag = "ORG.Position"
)]
pub async fn update_position(
    State(state): State<CommandState>,
    ctx: SecurityContext,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdatePositionReq>,
) -> Result<ApiOk<UpdatePositionRes>, ApiError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let command = org_application::commands::PositionUpdateCommand {
        id,
        name: req.name,
        code: req.code,
        remark: req.remark,
        operator_id: Some(ctx.id()),
    };

    org_application::commands::handle_position_update(&*state.uow_factory, &*state.clock, command)
        .await
        .map_err(AppError::from)?;

    Ok(ApiOk::data(UpdatePositionRes {}))
}
