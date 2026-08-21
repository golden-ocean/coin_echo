use axum::extract::{Query, State};
use platform_kernel::http::{PaginatedResponse, PaginationParams};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

use sys_application::error::AppError;

use crate::{
    response::{ApiError, ApiOk},
    state::QueryState,
};

// =========================================================================
// 字典项分页查询 (List DictionaryItem，按 dictionary_id 过滤)
// =========================================================================
#[derive(Debug, Deserialize, Validate, IntoParams, ToSchema)]
pub struct PageDictionaryItemParams {
    #[param(example = "01a023e6-ee82-79f1-96e5-57365294cd4f")]
    dictionary_id: Uuid,
    #[param(example = "1")]
    #[validate(range(min = 1, message = "页码必须从 1 开始"))]
    page: Option<u32>,
    #[param(example = "10")]
    #[validate(range(min = 1, max = 100, message = "每页条数必须在 1-100 之间"))]
    per_page: Option<u32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PageDictionaryItemRes {
    pub id: Uuid,
    pub label: String,
    pub value: String,
    pub color: Option<String>,
    pub is_builtin: bool,
    pub sort: i32,
    pub status: String,
}

impl From<sys_application::queries::DictionaryItemPageItem> for PageDictionaryItemRes {
    fn from(item: sys_application::queries::DictionaryItemPageItem) -> Self {
        Self {
            id: item.id,
            label: item.label,
            value: item.value,
            color: item.color,
            is_builtin: item.is_builtin,
            sort: item.sort,
            status: item.status,
        }
    }
}

#[utoipa::path(
    get,
    path = "",
    params(PageDictionaryItemParams),
    responses(
        (status = 200, description = "字典项分页列表", body = ApiOk<PaginatedResponse<PageDictionaryItemRes>>),
    ),
    tag = "SYS.DictionaryItem"
)]
pub async fn page_dictionary_item(
    State(state): State<QueryState>,
    Query(params): Query<PageDictionaryItemParams>,
) -> Result<ApiOk<PaginatedResponse<PageDictionaryItemRes>>, ApiError> {
    params
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let pagination = PaginationParams::new(
        params.page.unwrap_or(1),
        params
            .per_page
            .unwrap_or(PaginationParams::DEFAULT_PER_PAGE),
    );

    let query = sys_application::queries::DictionaryItemPageQuery {
        dictionary_id: params.dictionary_id,
        pagination,
    };

    let (items, total) =
        sys_application::queries::handle_dictionary_item_page(&state.reader_pool, query)
            .await
            .map_err(AppError::from)?;

    let response = PaginatedResponse::new(items, pagination)
        .with_total(total)
        .map(PageDictionaryItemRes::from);

    Ok(ApiOk::data(response))
}
