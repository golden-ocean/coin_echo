use sqlx::PgPool;
use uuid::Uuid;

use crate::{error::AppError, queries::QueryError};

pub struct DictionaryListItem {
    pub id: Uuid,
    pub name: String,
    pub code: String,
    pub is_builtin: bool,
    pub sort: i32,
    pub status: String,
    pub item_count: i64,
}

pub async fn handle_dictionary_list(pool: &PgPool) -> Result<Vec<DictionaryListItem>, AppError> {
    // 左表 + 子项计数一次查完，避免前端为了显示"每个字典下有几项"而对每个字典再单独发一次请求
    let rows = sqlx::query_as!(
        DictionaryListItem,
        r#"
            SELECT
                d.id, d.name, d.code, d.is_builtin, d.sort,
                d.status,
                COUNT(i.id) AS "item_count!"
            FROM sys_dictionary d
            LEFT JOIN sys_dictionary_item i ON i.dictionary_id = d.id
            GROUP BY d.id
            ORDER BY d.sort ASC, d.created_at ASC
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(QueryError::from)?;

    Ok(rows)
}
