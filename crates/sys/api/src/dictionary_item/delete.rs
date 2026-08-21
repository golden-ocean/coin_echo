use axum::extract::{Path, State};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use sys_application::error::AppError;

use crate::{
    response::{ApiError, ApiOk},
    state::CommandState,
};

// =========================================================================
// 字典项删除 (Delete)
// =========================================================================
#[derive(Debug, Serialize, ToSchema)]
pub struct DeleteDictionaryItemRes {}

#[utoipa::path(
    delete,
    path = "/{id}",
    params(("id" = Uuid, Path, description = "字典项ID", example = "018f3d61-9c12-7bb3-a00d-5a81e9f1a234")),
    responses(
        (status = 200, description = "字典项删除成功"),
        (status = 403, description = "内置字典项不可删除"),
        (status = 404, description = "字典项不存在"),
    ),
    tag = "SYS.DictionaryItem"
)]
pub async fn delete_dictionary_item(
    State(state): State<CommandState>,
    Path(id): Path<Uuid>,
) -> Result<ApiOk<DeleteDictionaryItemRes>, ApiError> {
    let command = sys_application::commands::DictionaryItemDeleteCommand { id };

    sys_application::commands::handle_dictionary_item_delete(&*state.uow_factory, command)
        .await
        .map_err(AppError::from)?;

    Ok(ApiOk::data(DeleteDictionaryItemRes {}))
}
