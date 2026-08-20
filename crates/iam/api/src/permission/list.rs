use axum::extract::{Query, State};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

use iam_application::error::AppError;

use crate::{
    response::{ApiError, ApiOk},
    state::QueryState,
};

// =========================================================================
// 权限全量列表查询 (List Permission)
// =========================================================================
// 注意：与角色列表不同，这里刻意不做分页。权限数据是配置型数据（总量通常几十到
// 几百条），且前端需要拿到完整数据集来拼装权限树（勾选树、动态路由生成等），
// 分页会导致父子节点碎片化、前端无法正确构建树结构。分页交给前端在渲染层做
// （展开/折叠），后端只做条件过滤。

#[derive(Debug, Deserialize, Validate, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct ListPermissionReq {
    /// 模糊搜索，同时匹配权限名称和权限编码
    #[param(example = "用户")]
    #[validate(length(min = 1, max = 128, message = "关键字长度必须在 1-128 之间"))]
    keyword: Option<String>,

    /// 精确过滤：menu / button / api
    #[param(example = "api")]
    kind: Option<String>,

    /// 精确过滤：enabled / disabled
    #[param(example = "enabled")]
    status: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListPermissionRes {
    #[schema(example = "018f3d61-9c12-7bb3-a00d-5a81e9f1a234")]
    pub id: String,
    #[schema(example = "018f3d61-9c12-7bb3-a00d-5a81e9f1a234")]
    pub parent_id: Option<String>,
    #[schema(example = "新增用户")]
    pub name: String,
    #[schema(example = "iam:user:add")]
    pub code: String,
    #[schema(example = "api")]
    pub kind: String,
    #[schema(example = "/system/user")]
    pub route_path: Option<String>,
    #[schema(example = "views/system/user/index")]
    pub component: Option<String>,
    #[schema(example = "user-icon")]
    pub icon: Option<String>,
    #[schema(example = "POST")]
    pub api_method: Option<String>,
    #[schema(example = "/api/v1/users")]
    pub api_path: Option<String>,
    pub is_builtin: bool,
    #[schema(example = 10)]
    pub sort: i32,
    #[schema(example = "enabled")]
    pub status: String,
}

#[utoipa::path(
    get,
    path = "",
    params(ListPermissionReq),
    responses(
        (status = 200, description = "权限全量列表（非分页，用于前端拼装权限树）", body = Vec<ListPermissionRes>),
        (status = 400, description = "请求参数校验失败"),
        (status = 401, description = "未认证"),
        (status = 403, description = "无权限"),
        (status = 500, description = "服务内部错误"),
    ),
    tag = "IAM.Permission"
)]
pub async fn list_permission(
    State(state): State<QueryState>,
    Query(req): Query<ListPermissionReq>,
) -> Result<ApiOk<Vec<ListPermissionRes>>, ApiError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let query = iam_application::queries::PermissionListQuery {
        keyword: req.keyword,
        kind: req.kind,
        status: req.status,
    };

    let records = iam_application::queries::handle_permission_list(&state.reader_pool, &query)
        .await
        .map_err(AppError::from)?;

    let items = records
        .into_iter()
        .map(|item| ListPermissionRes {
            id: item.id.to_string(),
            parent_id: item.parent_id.map(|id| id.to_string()),
            name: item.name,
            code: item.code,
            kind: item.kind,
            route_path: item.route_path,
            component: item.component,
            icon: item.icon,
            api_method: item.api_method,
            api_path: item.api_path,
            is_builtin: item.is_builtin,
            sort: item.sort,
            status: item.status,
        })
        .collect();

    Ok(ApiOk::data(items))
}
