use axum::extract::{Path, State};
use serde::Serialize;
use sys_application::error::AppError;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    response::{ApiError, ApiOk},
    state::CommandState,
};

// =========================================================================
// 字典删除 (Delete Dictionary)
// =========================================================================
#[derive(Debug, Serialize, ToSchema)]
pub struct DeleteDictionaryRes {}

#[utoipa::path(
    delete,
    path = "/{id}",
    params(("id" = Uuid, Path, description = "字典ID", example = "018f3d61-9c12-7bb3-a00d-5a81e9f1a234")),
    responses(
        (status = 200, description = "字典删除成功"),
        (status = 403, description = "内置字典不可删除，或字典下仍有字典项"),
        (status = 404, description = "字典不存在"),
    ),
    tag = "SYS.Dictionary"
)]
pub async fn delete_dictionary(
    State(state): State<CommandState>,
    Path(id): Path<Uuid>,
) -> Result<ApiOk<DeleteDictionaryRes>, ApiError> {
    let command = sys_application::commands::DictionaryDeleteCommand { id };

    sys_application::commands::handle_dictionary_delete(&*state.uow_factory, command)
        .await
        .map_err(AppError::from)?;

    Ok(ApiOk::data(DeleteDictionaryRes {}))
}
