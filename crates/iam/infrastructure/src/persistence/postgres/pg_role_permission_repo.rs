use std::collections::HashSet;

use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use iam_application::ports::{PortError, RolePermissionRepository};
use iam_domain::id::{PermissionId, RoleId};

pub struct PgRolePermissionRepository<'tx, 'c> {
    tx: &'tx mut Transaction<'c, Postgres>,
}

impl<'tx, 'c> PgRolePermissionRepository<'tx, 'c> {
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
impl<'tx, 'c> RolePermissionRepository for PgRolePermissionRepository<'tx, 'c> {
    async fn replace_permissions(
        &mut self,
        role_id: &RoleId,
        permission_ids: &[PermissionId],
    ) -> Result<(), PortError> {
        let role_uuid = role_id.as_uuid();

        // 1. 拉取数据库当前的历史快照
        let existing_rows = sqlx::query!(
            "SELECT permission_id FROM iam_role_permission WHERE role_id = $1",
            role_uuid
        )
        .fetch_all(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        let existing_set: HashSet<Uuid> =
            existing_rows.into_iter().map(|r| r.permission_id).collect();
        let target_set: HashSet<Uuid> = permission_ids.iter().map(|id| id.as_uuid()).collect();

        // 2. 求对称差集，只处理真正变化的部分
        let to_delete: Vec<Uuid> = existing_set.difference(&target_set).cloned().collect();
        let to_insert: Vec<Uuid> = target_set.difference(&existing_set).cloned().collect();

        if !to_delete.is_empty() {
            sqlx::query!(
                "DELETE FROM iam_role_permission WHERE role_id = $1 AND permission_id = ANY($2)",
                role_uuid,
                &to_delete
            )
            .execute(&mut **self.tx)
            .await
            .map_err(Self::map_sqlx_error)?;
        }

        if !to_insert.is_empty() {
            sqlx::query!(
                r#"
                    INSERT INTO iam_role_permission (role_id, permission_id)
                    SELECT $1, * FROM UNNEST($2::uuid[])
                "#,
                role_uuid,
                &to_insert
            )
            .execute(&mut **self.tx)
            .await
            .map_err(Self::map_sqlx_error)?;
        }

        Ok(())
    }

    async fn list_permission_ids_by_role(
        &mut self,
        role_id: &RoleId,
    ) -> Result<Vec<PermissionId>, PortError> {
        let rows = sqlx::query_scalar!(
            r#"SELECT permission_id FROM iam_role_permission WHERE role_id = $1"#,
            role_id.as_uuid()
        )
        .fetch_all(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(rows.into_iter().map(PermissionId::from_uuid).collect())
    }

    async fn list_role_ids_by_permission(
        &mut self,
        permission_id: &PermissionId,
    ) -> Result<Vec<RoleId>, PortError> {
        let rows = sqlx::query_scalar!(
            r#"SELECT role_id FROM iam_role_permission WHERE permission_id = $1"#,
            permission_id.as_uuid()
        )
        .fetch_all(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(rows.into_iter().map(RoleId::from_uuid).collect())
    }
}
