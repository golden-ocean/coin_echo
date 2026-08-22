use axum::extract::{Path, State};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use org_application::error::AppError;

use crate::{
    response::{ApiError, ApiOk},
    state::CommandState,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct DeleteOrganizationRes {}

#[utoipa::path(
    delete,
    path = "/{id}",
    params(("id" = Uuid, Path, description = "组织ID")),
    responses(
        (status = 200, description = "组织删除成功"),
        (status = 404, description = "组织不存在"),
        (status = 409, description = "组织下仍有子组织或成员，无法删除"),
    ),
    tag = "ORG.Organization"
)]
pub async fn delete_organization(
    State(state): State<CommandState>,
    Path(id): Path<Uuid>,
) -> Result<ApiOk<DeleteOrganizationRes>, ApiError> {
    let command = org_application::commands::OrganizationDeleteCommand { id };

    org_application::commands::handle_organization_delete(
        &*state.uow_factory,
        &*state.membership_checker,
        command,
    )
    .await
    .map_err(AppError::from)?;

    Ok(ApiOk::data(DeleteOrganizationRes {}))
}
