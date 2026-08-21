use axum::extract::State;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use sys_application::error::AppError;

use crate::{
    response::{ApiError, ApiOk},
    state::QueryState,
};

// =========================================================================
// 字典列表查询 (List Dictionary，全量)
// =========================================================================
#[derive(Debug, Serialize, ToSchema)]
pub struct ListDictionaryRes {
    pub id: Uuid,
    pub name: String,
    pub code: String,
    pub is_builtin: bool,
    pub sort: i32,
    pub status: String,
    pub item_count: i64,
}

impl From<sys_application::queries::DictionaryListItem> for ListDictionaryRes {
    fn from(item: sys_application::queries::DictionaryListItem) -> Self {
        Self {
            id: item.id,
            name: item.name,
            code: item.code,
            is_builtin: item.is_builtin,
            sort: item.sort,
            status: item.status,
            item_count: item.item_count,
        }
    }
}

#[utoipa::path(
    get,
    path = "",
    responses(
        (status = 200, description = "字典列表（全量）", body = ApiOk<Vec<ListDictionaryRes>>),
    ),
    tag = "SYS.Dictionary"
)]
pub async fn list_dictionary(
    State(state): State<QueryState>,
) -> Result<ApiOk<Vec<ListDictionaryRes>>, ApiError> {
    let items = sys_application::queries::handle_dictionary_list(&state.reader_pool)
        .await
        .map_err(AppError::from)?;

    let res = items.into_iter().map(ListDictionaryRes::from).collect();
    Ok(ApiOk::data(res))
}
