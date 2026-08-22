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
pub struct UpdateOrganizationReq {
    #[validate(length(min = 1, max = 64))]
    name: String,
    #[validate(length(min = 1, max = 64))]
    code: String,
    #[validate(length(max = 64))]
    contact: Option<String>,
    #[validate(length(max = 32))]
    phone: Option<String>,
    #[validate(email(message = "邮箱格式不正确"))]
    email: Option<String>,
    #[validate(length(max = 500))]
    remark: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UpdateOrganizationRes {}

#[utoipa::path(
    put,
    path = "/{id}",
    request_body = UpdateOrganizationReq,
    params(("id" = Uuid, Path, description = "组织ID")),
    responses(
        (status = 200, description = "组织信息更新成功"),
        (status = 400, description = "参数校验失败"),
        (status = 404, description = "组织不存在"),
        (status = 409, description = "组织名称/编码 已存在"),
    ),
    tag = "ORG.Organization"
)]
pub async fn update_organization(
    State(state): State<CommandState>,
    ctx: SecurityContext,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateOrganizationReq>,
) -> Result<ApiOk<UpdateOrganizationRes>, ApiError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let command = org_application::commands::OrganizationUpdateCommand {
        id,
        name: req.name,
        code: req.code,
        contact: req.contact,
        phone: req.phone,
        email: req.email,
        remark: req.remark,
        operator_id: Some(ctx.id()),
    };

    org_application::commands::handle_organization_update(
        &*state.uow_factory,
        &*state.clock,
        command,
    )
    .await
    .map_err(AppError::from)?;

    Ok(ApiOk::data(UpdateOrganizationRes {}))
}
