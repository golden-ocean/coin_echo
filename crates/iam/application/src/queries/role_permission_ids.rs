use crate::{error::AppError, queries::QueryError};
use sqlx::PgPool;
use uuid::Uuid;

/// 查询某角色当前拥有的权限 ID 列表（用于前端权限树勾选回显）
///
/// 注意：这是纯读路径，不走 UnitOfWork（无需事务），直接用只读连接池查询，
/// 和其他 queries 模块（role_page / user_page / permission_list）保持一致的风格。
pub async fn handle_role_permission_ids(
    pool: &PgPool,
    role_id: Uuid,
) -> Result<Vec<Uuid>, AppError> {
    let rows = sqlx::query_scalar!(
        r#"SELECT permission_id FROM iam_role_permission WHERE role_id = $1"#,
        role_id
    )
    .fetch_all(pool)
    .await
    .map_err(QueryError::from)?;

    Ok(rows)
}

#[cfg(test)]
mod tests {
    // 建议用 sqlx::test 集成测试覆盖：
    // - 角色有权限时，返回正确的权限 ID 集合
    // - 角色没有任何权限时，返回空 Vec 而不是报错
    // - role_id 对应的角色本身不存在时，同样返回空 Vec（查询层不校验角色是否存在，
    //   角色是否存在的校验属于需要该保证的调用方职责）
}
