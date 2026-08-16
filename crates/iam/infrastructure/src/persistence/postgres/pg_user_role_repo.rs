use std::collections::HashSet;

use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use iam_application::ports::{PortError, UserRoleRepository};
use iam_domain::id::{RoleId, UserId};

pub struct PgUserRoleRepository<'tx, 'c> {
    tx: &'tx mut Transaction<'c, Postgres>,
}

impl<'tx, 'c> PgUserRoleRepository<'tx, 'c> {
    pub fn new(tx: &'tx mut Transaction<'c, Postgres>) -> Self {
        Self { tx }
    }

    fn map_sqlx_error(e: sqlx::Error) -> PortError {
        if matches!(e, sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed) {
            return PortError::Infrastructure(e.to_string());
        }
        PortError::Database
    }
}

#[async_trait::async_trait]
impl<'tx, 'c> UserRoleRepository for PgUserRoleRepository<'tx, 'c> {
    async fn replace_roles(
        &mut self,
        user_id: &UserId,
        role_ids: &[RoleId],
    ) -> Result<(), PortError> {
        let user_uuid = user_id.as_uuid();

        // 1. 拉取数据库当前的历史快照
        let existing_rows = sqlx::query!(
            "SELECT role_id FROM iam_user_role WHERE user_id = $1",
            user_uuid
        )
        .fetch_all(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        let existing_set: HashSet<Uuid> = existing_rows.into_iter().map(|r| r.role_id).collect();
        let target_set: HashSet<Uuid> = role_ids.iter().map(|id| id.as_uuid()).collect();

        // 2. 求对称差集：只处理真正变化的部分，未变的行完全不动
        let to_delete: Vec<Uuid> = existing_set.difference(&target_set).cloned().collect();
        let to_insert: Vec<Uuid> = target_set.difference(&existing_set).cloned().collect();

        // 3. 按差集精准执行；集合完全相同时（最常见的重复提交场景）
        //    这里会直接跳过，一条 DML 都不会发出
        if !to_delete.is_empty() {
            sqlx::query!(
                "DELETE FROM iam_user_role WHERE user_id = $1 AND role_id = ANY($2)",
                user_uuid,
                &to_delete
            )
            .execute(&mut **self.tx)
            .await
            .map_err(Self::map_sqlx_error)?;
        }

        if !to_insert.is_empty() {
            sqlx::query!(
                r#"
                    INSERT INTO iam_user_role (user_id, role_id)
                    SELECT $1, * FROM UNNEST($2::uuid[])
                "#,
                user_uuid,
                &to_insert
            )
            .execute(&mut **self.tx)
            .await
            .map_err(Self::map_sqlx_error)?;
        }

        Ok(())
    }

    async fn list_role_ids_by_user(&mut self, user_id: &UserId) -> Result<Vec<RoleId>, PortError> {
        let rows = sqlx::query_scalar!(
            r#"SELECT role_id FROM iam_user_role WHERE user_id = $1"#,
            user_id.as_uuid()
        )
        .fetch_all(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(rows.into_iter().map(RoleId::from_uuid).collect())
    }

    async fn list_user_ids_by_role(&mut self, role_id: &RoleId) -> Result<Vec<UserId>, PortError> {
        let rows = sqlx::query_scalar!(
            r#"SELECT user_id FROM iam_user_role WHERE role_id = $1"#,
            role_id.as_uuid()
        )
        .fetch_all(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(rows.into_iter().map(UserId::from_uuid).collect())
    }
}
