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
pub struct DeletePositionRes {}

#[utoipa::path(
    delete,
    path = "/{id}",
    params(("id" = Uuid, Path, description = "职位ID")),
    responses(
        (status = 200, description = "职位删除成功"),
        (status = 404, description = "职位不存在"),
        (status = 409, description = "职位仍有成员关联，无法删除"),
    ),
    tag = "ORG.Position"
)]
pub async fn delete_position(
    State(state): State<CommandState>,
    Path(id): Path<Uuid>,
) -> Result<ApiOk<DeletePositionRes>, ApiError> {
    let command = org_application::commands::PositionDeleteCommand { id };

    org_application::commands::handle_position_delete(
        &*state.uow_factory,
        &*state.membership_checker,
        command,
    )
    .await
    .map_err(AppError::from)?;

    Ok(ApiOk::data(DeletePositionRes {}))
}
