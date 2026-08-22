use sqlx::PgPool;
use uuid::Uuid;

use crate::{error::AppError, queries::error::QueryError};

#[derive(Debug, Clone)]
pub struct PositionListItem {
    pub id: Uuid,
    pub name: String,
    pub code: String,
    pub sort: i32,
    pub remark: Option<String>,
    pub status: String,
}

/// 职位表数据量与字典类似（全局定义，条目有限），全量返回不分页
pub async fn handle_position_list(pool: &PgPool) -> Result<Vec<PositionListItem>, AppError> {
    let items = sqlx::query_as!(
        PositionListItem,
        r#"
            SELECT id, name, code, sort, remark, status
            FROM org_position
            WHERE deleted_at IS NULL
            ORDER BY sort ASC, created_at ASC
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(QueryError::from)?;

    Ok(items)
}
