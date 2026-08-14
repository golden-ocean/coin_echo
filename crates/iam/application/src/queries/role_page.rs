use crate::{error::AppError, queries::QueryError};
use platform_kernel::http::PaginationParams;
use uuid::Uuid;

pub struct RolePageQuery {
    pub name: Option<String>,
    pub code: Option<String>,
    pub status: Option<String>,
    pub pagination: PaginationParams,
}

#[derive(Debug)]
pub struct RolePageItem {
    pub id: Uuid,
    pub name: String,
    pub code: String,
    pub sort: i32,
    pub remark: Option<String>,
    pub status: String,
}

pub async fn handle_role_page(
    pool: &sqlx::PgPool,
    query: &RolePageQuery,
) -> Result<(Vec<RolePageItem>, u64), AppError> {
    let limit = query.pagination.limit() as i64;
    let offset = query.pagination.offset() as i64;

    let total = sqlx::query_scalar!(
        r#"
            SELECT COUNT(1) as "count!"
            FROM iam_role
            WHERE deleted_at IS NULL
            AND ($1::varchar IS NULL OR name LIKE '%' || $1 || '%')
            AND ($2::varchar IS NULL OR code = $2)
            AND ($3::varchar IS NULL OR status = $3)
        "#,
        query.name,
        query.code,
        query.status
    )
    .fetch_one(pool)
    .await
    .map_err(QueryError::from)?;

    let rows = sqlx::query!(
        r#"
            SELECT id, name, code, sort, remark, status
            FROM iam_role
            WHERE deleted_at IS NULL
            AND ($1::varchar IS NULL OR name LIKE '%' || $1 || '%')
            AND ($2::varchar IS NULL OR code = $2)
            AND ($3::varchar IS NULL OR status = $3)
            ORDER BY created_at DESC
            LIMIT $4 OFFSET $5
        "#,
        query.name,
        query.code,
        query.status,
        limit,
        offset
    )
    .fetch_all(pool)
    .await
    .map_err(QueryError::from)?;

    let items: Vec<RolePageItem> = rows
        .into_iter()
        .map(|r| RolePageItem {
            id: r.id,
            name: r.name,
            code: r.code,
            sort: r.sort,
            remark: r.remark,
            status: r.status,
        })
        .collect();

    Ok((items, total as u64))
}
