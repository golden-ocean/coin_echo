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
pub struct OrganizationTreeNodeRes {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub code: String,
    pub contact: String,
    pub phone: String,
    pub email: String,
    pub sort: i32,
    pub remark: Option<String>,
    pub status: String,
    #[schema(value_type = Vec<Object>)]
    pub children: Vec<OrganizationTreeNodeRes>,
}

impl From<org_application::queries::OrganizationTreeNode> for OrganizationTreeNodeRes {
    fn from(node: org_application::queries::OrganizationTreeNode) -> Self {
        Self {
            id: node.item.id,
            parent_id: node.item.parent_id,
            name: node.item.name,
            code: node.item.code,
            contact: node.item.contact,
            phone: node.item.phone,
            email: node.item.email,
            sort: node.item.sort,
            remark: node.item.remark,
            status: node.item.status,
            children: node.children.into_iter().map(Into::into).collect(),
        }
    }
}

#[utoipa::path(
    get,
    path = "",
    responses(
        (status = 200, description = "组织树（全量）", body = [OrganizationTreeNodeRes]),
    ),
    tag = "ORG.Organization"
)]
pub async fn list_organization(
    State(state): State<QueryState>,
) -> Result<ApiOk<Vec<OrganizationTreeNodeRes>>, ApiError> {
    let tree = org_application::queries::handle_organization_tree(&state.reader_pool)
        .await
        .map_err(AppError::from)?;

    let res = tree
        .into_iter()
        .map(OrganizationTreeNodeRes::from)
        .collect();
    Ok(ApiOk::data(res))
}
