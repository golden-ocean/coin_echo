use axum::extract::{Query, State};
use platform_kernel::http::{PaginatedResponse, PaginationParams};
use platform_security::context::SecurityContext;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

use iam_application::error::AppError;

use crate::{api_error::ApiError, api_res::ApiOk, state::QueryState};

// =========================================================================
// 角色多条件可选分页查询 (Get Role Page)
// =========================================================================
#[derive(Debug, Deserialize, Validate, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct PageRoleReq {
    #[param(example = "1")]
    #[validate(range(min = 1, message = "页码必须从 1 开始"))]
    page: Option<u32>,
    #[param(example = "10")]
    #[validate(range(min = 1, max = 100, message = "每页条数必须在 1-100 之间"))]
    per_page: Option<u32>,

    #[param(example = "user")]
    #[validate(length(min = 1, max = 50, message = "用户名长度必须在 1-50 之间"))]
    code: Option<String>,
    #[param(example = "user@example.com")]
    #[validate(length(min = 1, max = 128, message = "邮箱长度必须在 1-128 之间"))]
    name: Option<String>,
    #[param(example = "enabled")]
    status: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PageRoleRes {
    #[schema(example = "018f3d61-9c12-7bb3-a00d-5a81e9f1a234")]
    pub id: String,
    #[schema(example = "admin_user")]
    pub name: String,
    #[schema(example = "张三")]
    pub code: String,
    #[schema(example = 1000)]
    pub sort: i32,
    #[schema(example = "管理员备注")]
    pub remark: Option<String>,
    #[schema(example = "enabled")]
    pub status: String,
}

#[utoipa::path(
    get,
    path = "",
    params(PageRoleReq),
    responses(
        (status = 200, description = "角色列表分页"),
        (status = 400, description = "请求参数校验失败"),
        (status = 401, description = "未认证"),
        (status = 403, description = "无权限"),
        (status = 500, description = "服务内部错误"),
    ),
    tag = "IAM.Role"
)]
pub async fn page_role(
    State(state): State<QueryState>,
    ctx: SecurityContext,
    Query(req): Query<PageRoleReq>,
) -> Result<ApiOk<PaginatedResponse<PageRoleRes>>, ApiError<AppError>> {
    state
        .enforcer
        .check(&ctx.id().to_string(), "iam::role::page")
        .await
        .map_err(|_| ApiError::iam(AppError::Unauthorized))?;

    req.validate()
        .map_err(|e| ApiError::iam(AppError::Validation(e.to_string())))?;

    let pagination = PaginationParams::new(
        req.page.unwrap_or(1),
        req.per_page.unwrap_or(PaginationParams::DEFAULT_PER_PAGE),
    );

    let query = iam_application::queries::RolePageQuery {
        name: req.name,
        code: req.code,
        status: req.status,
        pagination,
    };

    let (records, total) = iam_application::queries::handle_role_page(&state.reader_pool, &query)
        .await
        .map_err(ApiError::iam)?;

    let page = PaginatedResponse::new(
        records
            .into_iter()
            .map(|item| PageRoleRes {
                id: item.id.to_string(),
                name: item.name,
                code: item.code,
                sort: item.sort,
                remark: item.remark,
                status: item.status,
            })
            .collect(),
        pagination,
    )
    .with_total(total);

    Ok(ApiOk::data(page))
}
