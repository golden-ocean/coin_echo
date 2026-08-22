use axum::{Json, extract::State};
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
pub struct CreateOrganizationReq {
    #[schema(example = "3c9e1a2e-1234-4a5b-9abc-1234567890ab")]
    parent_id: Option<Uuid>,
    #[validate(length(min = 1, max = 64, message = "组织名称长度必须在 1-64 之间"))]
    #[schema(example = "北京分公司")]
    name: String,
    #[validate(length(min = 1, max = 64, message = "组织编码长度必须在 1-64 之间"))]
    #[schema(example = "beijing")]
    code: String,
    #[validate(length(max = 64))]
    contact: Option<String>,
    #[validate(length(max = 32))]
    phone: Option<String>,
    #[validate(email(message = "邮箱格式不正确"))]
    email: Option<String>,
    #[validate(range(min = 0, max = 9999, message = "排序值必须在 0-9999 之间"))]
    sort: Option<i32>,
    #[validate(length(max = 500))]
    remark: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateOrganizationRes {}

#[utoipa::path(
    post,
    path = "",
    request_body = CreateOrganizationReq,
    responses(
        (status = 200, description = "组织创建成功"),
        (status = 400, description = "参数校验失败"),
        (status = 404, description = "父组织不存在"),
        (status = 409, description = "组织名称/编码 已存在"),
    ),
    tag = "ORG.Organization"
)]
pub async fn create_organization(
    State(state): State<CommandState>,
    ctx: SecurityContext,
    Json(req): Json<CreateOrganizationReq>,
) -> Result<ApiOk<CreateOrganizationRes>, ApiError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let command = org_application::commands::OrganizationCreateCommand {
        parent_id: req.parent_id,
        name: req.name,
        code: req.code,
        contact: req.contact,
        phone: req.phone,
        email: req.email,
        sort: req.sort,
        remark: req.remark,
        operator_id: Some(ctx.id()),
    };

    org_application::commands::handle_organization_create(
        &*state.uow_factory,
        &*state.clock,
        command,
    )
    .await
    .map_err(AppError::from)?;

    Ok(ApiOk::data(CreateOrganizationRes {}))
}
