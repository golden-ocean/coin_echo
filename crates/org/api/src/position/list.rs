use axum::extract::State;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use org_application::error::AppError;

use crate::{
    response::{ApiError, ApiOk},
    state::QueryState,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct PositionListItemRes {
    pub id: Uuid,
    pub name: String,
    pub code: String,
    pub sort: i32,
    pub remark: Option<String>,
    pub status: String,
}

impl From<org_application::queries::PositionListItem> for PositionListItemRes {
    fn from(item: org_application::queries::PositionListItem) -> Self {
        Self {
            id: item.id,
            name: item.name,
            code: item.code,
            sort: item.sort,
            remark: item.remark,
            status: item.status,
        }
    }
}

#[utoipa::path(
    get,
    path = "",
    responses(
        (status = 200, description = "职位列表（全量）", body = [PositionListItemRes]),
    ),
    tag = "ORG.Position"
)]
pub async fn list_position(
    State(state): State<QueryState>,
) -> Result<ApiOk<Vec<PositionListItemRes>>, ApiError> {
    let items = org_application::queries::handle_position_list(&state.reader_pool)
        .await
        .map_err(AppError::from)?;

    let res = items.into_iter().map(PositionListItemRes::from).collect();
    Ok(ApiOk::data(res))
}
