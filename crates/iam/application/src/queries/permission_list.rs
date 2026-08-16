use crate::{error::AppError, queries::QueryError};
use uuid::Uuid;

pub struct PermissionListQuery {
    pub keyword: Option<String>, // 模糊匹配 name / code
    pub kind: Option<String>,    // 精确过滤：menu / button / api
    pub status: Option<String>,  // 精确过滤：enabled / disabled
}

#[derive(Debug)]
pub struct PermissionListItem {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub code: String,
    pub kind: String,
    pub route_path: Option<String>,
    pub component: Option<String>,
    pub icon: Option<String>,
    pub api_method: Option<String>,
    pub api_path: Option<String>,
    pub is_builtin: bool,
    pub sort: i32,
    pub status: String,
}

pub async fn handle_permission_list(
    pool: &sqlx::PgPool,
    query: &PermissionListQuery,
) -> Result<Vec<PermissionListItem>, AppError> {
    let rows = sqlx::query!(
        r#"
            SELECT
                id, parent_id, name, code, kind,
                route_path, component, icon, api_method, api_path,
                is_builtin, sort, status
            FROM iam_permission
            WHERE deleted_at IS NULL
              AND ($1::varchar IS NULL OR name LIKE '%' || $1 || '%' OR code LIKE '%' || $1 || '%')
              AND ($2::varchar IS NULL OR kind = $2)
              AND ($3::varchar IS NULL OR status = $3)
            ORDER BY sort ASC, created_at ASC
        "#,
        query.keyword,
        query.kind,
        query.status
    )
    .fetch_all(pool)
    .await
    .map_err(QueryError::from)?;

    let items = rows
        .into_iter()
        .map(|r| PermissionListItem {
            id: r.id,
            parent_id: r.parent_id,
            name: r.name,
            code: r.code,
            kind: r.kind,
            route_path: r.route_path,
            component: r.component,
            icon: r.icon,
            api_method: r.api_method,
            api_path: r.api_path,
            is_builtin: r.is_builtin,
            sort: r.sort,
            status: r.status,
        })
        .collect();

    Ok(items)
}
