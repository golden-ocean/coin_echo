use sqlx::PgPool;
use uuid::Uuid;

use platform_kernel::http::PaginationParams;

use crate::{error::AppError, queries::QueryError};

pub struct DictionaryItemPageItem {
    pub id: Uuid,
    pub label: String,
    pub value: String,
    pub color: Option<String>,
    pub is_builtin: bool,
    pub sort: i32,
    pub status: String,
}

pub struct DictionaryItemPageQuery {
    pub dictionary_id: Uuid,
    pub pagination: PaginationParams,
}

pub async fn handle_dictionary_item_page(
    pool: &PgPool,
    query: DictionaryItemPageQuery,
) -> Result<(Vec<DictionaryItemPageItem>, u64), AppError> {
    let limit = query.pagination.limit() as i64;
    let offset = query.pagination.offset() as i64;

    let items = sqlx::query_as!(
        DictionaryItemPageItem,
        r#"
            SELECT id, label, value, color, is_builtin, sort, status
            FROM sys_dictionary_item
            WHERE dictionary_id = $1
            ORDER BY sort ASC, created_at ASC
            LIMIT $2 OFFSET $3
        "#,
        query.dictionary_id,
        limit,
        offset,
    )
    .fetch_all(pool)
    .await
    .map_err(QueryError::from)?;

    let total: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!" FROM sys_dictionary_item WHERE dictionary_id = $1"#,
        query.dictionary_id
    )
    .fetch_one(pool)
    .await
    .map_err(QueryError::from)?;

    Ok((items, total as u64))
}
