use crate::{error::AppError, queries::QueryError};
use sqlx::PgPool;
use uuid::Uuid;

/// 查询某用户当前拥有的角色 ID 列表（用于前端角色多选框回显）
pub async fn handle_user_role_ids(pool: &PgPool, user_id: Uuid) -> Result<Vec<Uuid>, AppError> {
    let rows = sqlx::query_scalar!(
        r#"SELECT role_id FROM iam_user_role WHERE user_id = $1"#,
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(QueryError::from)?;

    Ok(rows)
}

#[cfg(test)]
mod tests {
    // 同 role_permission_ids 的测试思路：
    // - 用户有角色时返回正确集合
    // - 用户没有角色时返回空 Vec
}
