use axum::{
    Json,
    extract::{Path, State},
};
use platform_security::context::SecurityContext;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use org_application::error::AppError;

use crate::{
    response::{ApiError, ApiOk},
    state::CommandState,
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct MoveOrganizationReq {
    #[schema(example = "3c9e1a2e-1234-4a5b-9abc-1234567890ab")]
    new_parent_id: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MoveOrganizationRes {}

#[utoipa::path(
    put,
    path = "/{id}/move",
    request_body = MoveOrganizationReq,
    params(("id" = Uuid, Path, description = "组织ID")),
    responses(
        (status = 200, description = "组织移动成功"),
        (status = 400, description = "父节点非法（不能设为自己）"),
        (status = 404, description = "组织或新父组织不存在"),
    ),
    tag = "ORG.Organization"
)]
pub async fn move_organization(
    State(state): State<CommandState>,
    ctx: SecurityContext,
    Path(id): Path<Uuid>,
    Json(req): Json<MoveOrganizationReq>,
) -> Result<ApiOk<MoveOrganizationRes>, ApiError> {
    let command = org_application::commands::OrganizationMoveCommand {
        id,
        new_parent_id: req.new_parent_id,
        operator_id: Some(ctx.id()),
    };

    org_application::commands::handle_organization_move(
        &*state.uow_factory,
        &*state.clock,
        command,
    )
    .await
    .map_err(AppError::from)?;

    Ok(ApiOk::data(MoveOrganizationRes {}))
}
