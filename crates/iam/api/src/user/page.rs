use axum::extract::{Query, State};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

use iam_application::error::AppError;
use platform_kernel::http::{PaginatedResponse, PaginationParams};

use crate::{
    response::{ApiError, ApiOk},
    state::QueryState,
};

// =========================================================================
// Read 用户多条件可选分页查询 (Get User Page)
// =========================================================================
#[derive(Debug, Deserialize, Validate, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct PageUserReq {
    #[param(example = "1")]
    #[validate(range(min = 1, message = "页码必须从 1 开始"))]
    pub page: Option<i64>,
    #[param(example = "10")]
    #[validate(range(min = 1, max = 100, message = "每页条数必须在 1-100 之间"))]
    pub page_size: Option<i64>,

    #[param(example = "user")]
    #[validate(length(min = 1, max = 50, message = "用户名长度必须在 1-50 之间"))]
    pub username: Option<String>,
    #[param(example = "user@example.com")]
    #[validate(length(min = 1, max = 128, message = "邮箱长度必须在 1-128 之间"))]
    pub email: Option<String>,
    #[param(example = "enabled")]
    pub status: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PageUserRes {
    #[schema(example = "018f3d61-9c12-7bb3-a00d-5a81e9f1a234")]
    pub id: String,
    #[schema(example = "admin_user")]
    pub username: String,
    #[schema(example = "张三")]
    pub name: String,
    #[schema(example = "admin@example.com")]
    pub email: String,
    #[schema(example = "13800138000")]
    pub phone: String,
    #[schema(example = "enabled")]
    pub status: String,
}

#[utoipa::path(
    get,
    path = "",
    params(PageUserReq),
    responses(
        (status = 200, description = "用户列表分页"),
        (status = 400, description = "请求参数校验失败"),
    ),
    tag = "IAM.User"
)]
pub async fn page_user(
    State(state): State<QueryState>,
    Query(req): Query<PageUserReq>,
) -> Result<ApiOk<PaginatedResponse<PageUserRes>>, ApiError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let pagination = PaginationParams::new(
        req.page.unwrap_or(1) as u32,
        req.page_size
            .unwrap_or(PaginationParams::DEFAULT_PER_PAGE as i64) as u32,
    );

    let query = iam_application::queries::UserPageQuery {
        username: req.username,
        email: req.email,
        status: req.status,
        pagination,
    };

    let (records, total) = iam_application::queries::handle_user_page(&state.reader_pool, &query)
        .await
        .map_err(AppError::from)?;

    let paginated = PaginatedResponse::new(
        records
            .into_iter()
            .map(|item| PageUserRes {
                id: item.id.to_string(),
                username: item.username,
                name: item.name,
                email: item.email,
                phone: item.phone,
                status: item.status,
            })
            .collect(),
        pagination,
    )
    .with_total(total as u64);

    Ok(ApiOk::data(paginated))
}
