use sqlx::{Postgres, Transaction};

use iam_application::ports::{PortError, RoleRepository};
use iam_domain::{
    id::RoleId,
    role::{
        Role,
        value_object::{RoleCode, RoleName},
    },
};

use crate::persistence::models::RoleModel;

mod constraints {
    pub const CODE: &str = "uk_iam_role_code";
    pub const NAME: &str = "uk_iam_role_name";
}

pub struct PgRoleRepository<'tx, 'c> {
    tx: &'tx mut Transaction<'c, Postgres>,
}

impl<'tx, 'c> PgRoleRepository<'tx, 'c> {
    pub fn new(tx: &'tx mut Transaction<'c, Postgres>) -> Self {
        Self { tx }
    }

    fn map_sqlx_error(e: sqlx::Error) -> PortError {
        if let sqlx::Error::Database(db_err) = &e {
            if db_err.is_unique_violation() {
                return match db_err.constraint().unwrap_or_default() {
                    constraints::CODE => PortError::UniqueConflict {
                        entity: "role",
                        field: "code",
                    },
                    constraints::NAME => PortError::UniqueConflict {
                        entity: "role",
                        field: "name",
                    },
                    other => {
                        PortError::Infrastructure(format!("unknown unique constraint: {other}"))
                    }
                };
            }
            if db_err
                .code()
                .map(|c| c == "40001" || c == "40P01")
                .unwrap_or(false)
            {
                return PortError::VersionConflict { entity: "role" };
            }
        }
        if matches!(e, sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed) {
            return PortError::Infrastructure(e.to_string());
        }
        PortError::Database
    }
}

#[async_trait::async_trait]
impl<'tx, 'c> RoleRepository for PgRoleRepository<'tx, 'c> {
    // ── 写入操作 ────────────────────────────────────────────────
    async fn insert(&mut self, role: &Role) -> Result<(), PortError> {
        // 使用标准 From Trait 转换
        let m = RoleModel::from(role);

        sqlx::query!(
            r#"
                INSERT INTO iam_role (
                    id, name, code, is_builtin, sort, remark, status,
                    created_at, created_by, updated_at, updated_by, version
                ) VALUES (
                    $1,$2,$3,$4,$5,$6,$7,
                    $8,$9,$10,$11,$12
                )
            "#,
            m.id,
            m.name,
            m.code,
            m.is_builtin,
            m.sort,
            m.remark,
            m.status,
            m.created_at,
            m.created_by,
            m.updated_at,
            m.updated_by,
            m.version,
        )
        .execute(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(())
    }

    async fn update(&mut self, role: &Role) -> Result<(), PortError> {
        let m = RoleModel::from(role);
        let old_version = m.version - 1;

        let result = sqlx::query!(
            r#"
                UPDATE iam_role SET
                    name = $2, code = $3, sort = $4,
                    remark = $5, status = $6,
                    updated_at = $7, updated_by = $8, version = $9
                WHERE id = $1 AND version = $10 AND deleted_at IS NULL
            "#,
            m.id,
            m.name,
            m.code,
            m.sort,
            m.remark,
            m.status,
            m.updated_at,
            m.updated_by,
            m.version,
            old_version
        )
        .execute(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(PortError::VersionConflict { entity: "role" });
        }
        Ok(())
    }

    async fn soft_delete(&mut self, role: &Role) -> Result<(), PortError> {
        let m = RoleModel::from(role);
        let old_version = m.version - 1;

        let result = sqlx::query!(
            r#"
                UPDATE iam_role SET
                    deleted_at = $2, deleted_by = $3,
                    updated_at = $4, updated_by = $5, version = $6
                WHERE id = $1 AND version = $7 AND deleted_at IS NULL
            "#,
            m.id,
            m.deleted_at,
            m.deleted_by,
            m.updated_at,
            m.updated_by,
            m.version,
            old_version,
        )
        .execute(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(PortError::VersionConflict { entity: "role" });
        }
        Ok(())
    }

    // ── 聚合根查询 ───────────────
    async fn find_by_id(&mut self, id: &RoleId) -> Result<Option<Role>, PortError> {
        let row = sqlx::query_as!(
            RoleModel,
            r#"
                SELECT
                    id, name, code, is_builtin, sort, remark, status,
                    created_at, created_by, updated_at, updated_by,
                    deleted_at, deleted_by, version
                FROM iam_role
                WHERE id = $1 AND deleted_at IS NULL
            "#,
            id.as_uuid()
        )
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        row.map(|m| Role::try_from(&m)).transpose()
    }

    async fn find_by_code(&mut self, code: &RoleCode) -> Result<Option<Role>, PortError> {
        let row = sqlx::query_as!(
            RoleModel,
            r#"
                SELECT
                    id, name, code, is_builtin, sort, remark, status,
                    created_at, created_by, updated_at, updated_by,
                    deleted_at, deleted_by, version
                FROM iam_role
                WHERE code = $1 AND deleted_at IS NULL
            "#,
            code.as_str()
        )
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        row.map(|m| Role::try_from(&m)).transpose()
    }

    async fn find_by_name(&mut self, name: &RoleName) -> Result<Option<Role>, PortError> {
        let row = sqlx::query_as!(
            RoleModel,
            r#"
                SELECT
                    id, name, code, is_builtin, sort, remark, status,
                    created_at, created_by, updated_at, updated_by,
                    deleted_at, deleted_by, version
                FROM iam_role
                WHERE name = $1 AND deleted_at IS NULL
            "#,
            name.as_str()
        )
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        // 使用标准的 TryFrom<&RoleModel> 进行类型转换
        row.map(|m| Role::try_from(&m)).transpose()
    }

    // ── 存在性检查 (query_scalar! 零结构体) ────────────────────
    async fn exists_by_code(&mut self, code: &RoleCode) -> Result<bool, PortError> {
        let result = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM iam_role WHERE code = $1 AND deleted_at IS NULL) as "exists!""#,
            code.as_str()
        )
        .fetch_one(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(result)
    }

    async fn exists_by_name(&mut self, name: &RoleName) -> Result<bool, PortError> {
        let result = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM iam_role WHERE name = $1 AND deleted_at IS NULL) as "exists!""#,
            name.as_str()
        )
        .fetch_one(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(result)
    }
}
