use crate::{error::AppError, queries::QueryError};
use platform_kernel::http::PaginationParams;
use uuid::Uuid;

pub struct UserPageQuery {
    pub username: Option<String>,
    pub email: Option<String>,
    pub status: Option<String>,
    pub pagination: PaginationParams,
}

#[derive(Debug)]
pub struct UserPageItem {
    pub id: Uuid,
    pub username: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub status: String,
}

pub async fn handle_user_page(
    pool: &sqlx::PgPool,
    query: &UserPageQuery,
) -> Result<(Vec<UserPageItem>, i64), AppError> {
    let limit = query.pagination.per_page() as i64;
    let offset = query.pagination.page() as i64;

    // 利用 ($1::varchar IS NULL OR field = $1) 机制配合 UK 索引，彻底免除拼装字符串的隐患
    let total = sqlx::query_scalar!(
        r#"
            SELECT COUNT(1) as "count!"
            FROM iam_user
            WHERE deleted_at IS NULL
            AND ($1::varchar IS NULL OR username LIKE '%' || $1 || '%')
            AND ($2::varchar IS NULL OR email = $2)
            AND ($3::varchar IS NULL OR status = $3)
        "#,
        query.username,
        query.email,
        query.status
    )
    .fetch_one(pool)
    .await
    .map_err(QueryError::from)?;

    let rows = sqlx::query!(
        r#"
            SELECT id, username, name, email, phone, status
            FROM iam_user
            WHERE deleted_at IS NULL
            AND ($1::varchar IS NULL OR username LIKE '%' || $1 || '%')
            AND ($2::varchar IS NULL OR email = $2)
            AND ($3::varchar IS NULL OR status = $3)
            ORDER BY created_at DESC
            LIMIT $4 OFFSET $5
        "#,
        query.username,
        query.email,
        query.status,
        limit,
        offset
    )
    .fetch_all(pool)
    .await
    .map_err(QueryError::from)?;

    let items: Vec<UserPageItem> = rows
        .into_iter()
        .map(|r| UserPageItem {
            id: r.id,
            username: r.username,
            name: r.name,
            email: r.email,
            phone: r.phone,
            status: r.status,
        })
        .collect();

    Ok((items, total))
}
