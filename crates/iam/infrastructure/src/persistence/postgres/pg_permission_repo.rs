use sqlx::{Postgres, Transaction};

use crate::persistence::models::PermissionModel;
use iam_application::ports::{PermissionRepository, PortError};
use iam_domain::{
    id::PermissionId,
    permission::{Permission, value_object::PermissionCode, value_object::PermissionName},
};

mod constraints {
    pub const CODE: &str = "uk_iam_permission_code";
    pub const NAME: &str = "uk_iam_permission_name";
}

/// PermissionRepository 的 PostgreSQL 基础设施层具体实现
pub struct PgPermissionRepository<'tx, 'c> {
    tx: &'tx mut Transaction<'c, Postgres>,
}

impl<'tx, 'c> PgPermissionRepository<'tx, 'c> {
    pub fn new(tx: &'tx mut Transaction<'c, Postgres>) -> Self {
        Self { tx }
    }

    fn map_sqlx_error(e: sqlx::Error) -> PortError {
        if let sqlx::Error::Database(db_err) = &e {
            if db_err.is_unique_violation() {
                return match db_err.constraint().unwrap_or_default() {
                    constraints::CODE => PortError::UniqueConflict {
                        entity: "permission",
                        field: "code",
                    },
                    constraints::NAME => PortError::UniqueConflict {
                        entity: "permission",
                        field: "name",
                    },
                    other => {
                        PortError::Infrastructure(format!("unknown unique constraint: {other}"))
                    }
                };
            }
        }
        if matches!(e, sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed) {
            return PortError::Infrastructure(e.to_string());
        }
        PortError::Database
    }
}

#[async_trait::async_trait]
impl<'tx, 'c> PermissionRepository for PgPermissionRepository<'tx, 'c> {
    // ── 写入操作 ────────────────────────────────────────────────
    async fn insert(&mut self, permission: &Permission) -> Result<(), PortError> {
        let m = PermissionModel::from(permission);
        sqlx::query!(
            r#"
                INSERT INTO iam_permission (
                    id, parent_id, name, code, kind,
                    route_path, component, icon, api_method, api_path,
                    is_builtin, remark, sort, status,
                    created_at, created_by, updated_at, updated_by
                ) VALUES (
                    $1,$2,$3,$4,$5,
                    $6,$7,$8,$9,$10,
                    $11,$12,$13,$14,
                    $15,$16,$17,$18
                )
            "#,
            m.id,
            m.parent_id,
            m.name,
            m.code,
            m.kind,
            m.route_path,
            m.component,
            m.icon,
            m.api_method,
            m.api_path,
            m.is_builtin,
            m.remark,
            m.sort,
            m.status,
            m.created_at,
            m.created_by,
            m.updated_at,
            m.updated_by,
        )
        .execute(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(())
    }

    async fn update(&mut self, permission: &Permission) -> Result<(), PortError> {
        let m = PermissionModel::from(permission);
        let result = sqlx::query!(
            r#"
                UPDATE iam_permission SET
                    parent_id = $2, name = $3, code = $4, kind = $5,
                    route_path = $6, component = $7, icon = $8, api_method = $9, api_path = $10,
                    remark = $11, sort = $12, status = $13,
                    updated_at = $14, updated_by = $15
                WHERE id = $1 AND deleted_at IS NULL
            "#,
            m.id,
            m.parent_id,
            m.name,
            m.code,
            m.kind,
            m.route_path,
            m.component,
            m.icon,
            m.api_method,
            m.api_path,
            m.remark,
            m.sort,
            m.status,
            m.updated_at,
            m.updated_by,
        )
        .execute(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(PortError::NotFound {
                entity: "permission",
            });
        }
        Ok(())
    }

    async fn soft_delete(&mut self, permission: &Permission) -> Result<(), PortError> {
        let m = PermissionModel::from(permission);
        let result = sqlx::query!(
            r#"
                UPDATE iam_permission SET
                    deleted_at = $2, deleted_by = $3,
                    updated_at = $4, updated_by = $5
                WHERE id = $1 AND deleted_at IS NULL
            "#,
            m.id,
            m.deleted_at,
            m.deleted_by,
            m.updated_at,
            m.updated_by,
        )
        .execute(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(PortError::NotFound {
                entity: "permission",
            });
        }
        Ok(())
    }

    // ── 聚合根查询 ───────────────────────────────────────────
    async fn find_by_id(
        &mut self,
        permission_id: &PermissionId,
    ) -> Result<Option<Permission>, PortError> {
        let row = sqlx::query_as!(
            PermissionModel,
            r#"
                SELECT
                    id, parent_id, name, code, kind,
                    route_path, component, icon, api_method, api_path,
                    is_builtin, remark, sort, status,
                    created_at, created_by, updated_at, updated_by,
                    deleted_at, deleted_by
                FROM iam_permission
                WHERE id = $1 AND deleted_at IS NULL
            "#,
            permission_id.as_uuid()
        )
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_code(
        &mut self,
        code: &PermissionCode,
    ) -> Result<Option<Permission>, PortError> {
        let row = sqlx::query_as!(
            PermissionModel,
            r#"
                SELECT
                    id, parent_id, name, code, kind,
                    route_path, component, icon, api_method, api_path,
                    is_builtin, remark, sort, status,
                    created_at, created_by, updated_at, updated_by,
                    deleted_at, deleted_by
                FROM iam_permission
                WHERE code = $1 AND deleted_at IS NULL
            "#,
            code.as_str()
        )
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        row.map(TryInto::try_into).transpose()
    }

    // ── 存在性检查 (query_scalar! 零结构体) ────────────────────
    async fn exists_by_code(&mut self, code: &PermissionCode) -> Result<bool, PortError> {
        let result = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM iam_permission WHERE code = $1 AND deleted_at IS NULL) as "exists!""#,
            code.as_str()
        )
        .fetch_one(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(result)
    }

    async fn exists_by_name(&mut self, name: &PermissionName) -> Result<bool, PortError> {
        let result = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM iam_permission WHERE name = $1 AND deleted_at IS NULL) as "exists!""#,
            name.as_str()
        )
        .fetch_one(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(result)
    }

    // ── 树形结构查询 ────────────────────────────────────────────
    async fn find_by_parent_id(
        &mut self,
        parent_id: Option<PermissionId>,
    ) -> Result<Vec<Permission>, PortError> {
        let parent_uuid = parent_id.map(|pid| pid.as_uuid());

        let rows = sqlx::query_as!(
            PermissionModel,
            r#"
                SELECT
                    id, parent_id, name, code, kind,
                    route_path, component, icon, api_method, api_path,
                    is_builtin, remark, sort, status,
                    created_at, created_by, updated_at, updated_by,
                    deleted_at, deleted_by
                FROM iam_permission
                WHERE deleted_at IS NULL
                  AND (
                      ($1::uuid IS NULL AND parent_id IS NULL)
                      OR parent_id = $1
                  )
                ORDER BY sort ASC, created_at ASC
            "#,
            parent_uuid
        )
        .fetch_all(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn has_children(&mut self, id: &PermissionId) -> Result<bool, PortError> {
        let result = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM iam_permission WHERE parent_id = $1 AND deleted_at IS NULL) as "exists!""#,
            id.as_uuid()
        )
        .fetch_one(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(result)
    }

    async fn is_ancestor(
        &mut self,
        ancestor_id: &PermissionId,
        descendant_id: &PermissionId,
    ) -> Result<bool, PortError> {
        let result = sqlx::query_scalar!(
            r#"
                WITH RECURSIVE ancestors AS (
                    SELECT id, parent_id
                    FROM iam_permission
                    WHERE id = $2 AND deleted_at IS NULL

                    UNION ALL

                    SELECT p.id, p.parent_id
                    FROM iam_permission p
                    INNER JOIN ancestors a ON p.id = a.parent_id
                    WHERE p.deleted_at IS NULL
                )
                SELECT EXISTS(SELECT 1 FROM ancestors WHERE id = $1) as "exists!"
            "#,
            ancestor_id.as_uuid(),
            descendant_id.as_uuid(),
        )
        .fetch_one(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(result)
    }
}
