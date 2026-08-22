use sqlx::PgPool;
use uuid::Uuid;

use org_application::ports::{MembershipChecker, PortError};

/// 基于共享数据库的实现：直接查询 iam_user 表
/// 如果未来 iam/org 拆分为独立服务，这里是替换成 RPC 调用的唯一改动点
pub struct PgMembershipChecker {
    pool: PgPool,
}

impl PgMembershipChecker {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn map_sqlx_error(e: sqlx::Error) -> PortError {
        if matches!(e, sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed) {
            return PortError::Infrastructure(e.to_string());
        }
        PortError::Database
    }
}

#[async_trait::async_trait]
impl MembershipChecker for PgMembershipChecker {
    async fn has_users_in_organization(&self, organization_id: Uuid) -> Result<bool, PortError> {
        let result = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM iam_user WHERE organization_id = $1 AND deleted_at IS NULL) as "exists!""#,
            organization_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(result)
    }

    async fn has_users_in_position(&self, position_id: Uuid) -> Result<bool, PortError> {
        let result = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM iam_user WHERE position_id = $1 AND deleted_at IS NULL) as "exists!""#,
            position_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(result)
    }
}
