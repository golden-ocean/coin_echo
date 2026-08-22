use sqlx::{Postgres, Transaction};

use crate::persistence::models::PositionModel;
use org_application::ports::{PortError, PositionRepository};
use org_domain::{
    id::PositionId,
    position::{
        Position,
        value_object::{PositionCode, PositionName},
    },
};

mod constraints {
    pub const CODE: &str = "uk_org_position_code";
    pub const NAME: &str = "uk_org_position_name";
}

/// PositionRepository 的 PostgreSQL 基础设施层具体实现
pub struct PgPositionRepository<'tx, 'c> {
    tx: &'tx mut Transaction<'c, Postgres>,
}

impl<'tx, 'c> PgPositionRepository<'tx, 'c> {
    pub fn new(tx: &'tx mut Transaction<'c, Postgres>) -> Self {
        Self { tx }
    }

    fn map_sqlx_error(e: sqlx::Error) -> PortError {
        if let sqlx::Error::Database(db_err) = &e {
            if db_err.is_unique_violation() {
                return match db_err.constraint().unwrap_or_default() {
                    constraints::CODE => PortError::UniqueConflict {
                        entity: "position",
                        field: "code",
                    },
                    constraints::NAME => PortError::UniqueConflict {
                        entity: "position",
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
impl<'tx, 'c> PositionRepository for PgPositionRepository<'tx, 'c> {
    // ── 写入操作 ────────────────────────────────────────────────
    async fn insert(&mut self, position: &Position) -> Result<(), PortError> {
        let m = PositionModel::from(position);

        sqlx::query!(
            r#"
                INSERT INTO org_position (
                    id, name, code, sort, remark, status,
                    created_at, created_by, updated_at, updated_by
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10
                )
            "#,
            m.id,
            m.name,
            m.code,
            m.sort,
            m.remark,
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

    async fn update(&mut self, position: &Position) -> Result<(), PortError> {
        let m = PositionModel::from(position);

        let result = sqlx::query!(
            r#"
                UPDATE org_position SET
                    name = $2, code = $3, sort = $4, remark = $5, status = $6,
                    updated_at = $7, updated_by = $8
                WHERE id = $1 AND deleted_at IS NULL
            "#,
            m.id,
            m.name,
            m.code,
            m.sort,
            m.remark,
            m.status,
            m.updated_at,
            m.updated_by,
        )
        .execute(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(PortError::NotFound { entity: "position" });
        }
        Ok(())
    }

    async fn soft_delete(&mut self, position: &Position) -> Result<(), PortError> {
        let m = PositionModel::from(position);

        let result = sqlx::query!(
            r#"
                UPDATE org_position SET
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
            return Err(PortError::NotFound { entity: "position" });
        }
        Ok(())
    }

    // ── 聚合根查询 ────────────────────────────────────────────
    async fn find_by_id(&mut self, id: &PositionId) -> Result<Option<Position>, PortError> {
        let row = sqlx::query_as!(
            PositionModel,
            r#"
                SELECT
                    id, name, code, sort, remark, status,
                    created_at, created_by, updated_at, updated_by, deleted_at, deleted_by
                FROM org_position
                WHERE id = $1 AND deleted_at IS NULL
            "#,
            id.as_uuid()
        )
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_code(&mut self, code: &PositionCode) -> Result<Option<Position>, PortError> {
        let row = sqlx::query_as!(
            PositionModel,
            r#"
                SELECT
                    id, name, code, sort, remark, status,
                    created_at, created_by, updated_at, updated_by, deleted_at, deleted_by
                FROM org_position
                WHERE code = $1 AND deleted_at IS NULL
            "#,
            code.as_str()
        )
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_name(&mut self, name: &PositionName) -> Result<Option<Position>, PortError> {
        let row = sqlx::query_as!(
            PositionModel,
            r#"
                SELECT
                    id, name, code, sort, remark, status,
                    created_at, created_by, updated_at, updated_by, deleted_at, deleted_by
                FROM org_position
                WHERE name = $1 AND deleted_at IS NULL
            "#,
            name.as_str()
        )
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        row.map(TryInto::try_into).transpose()
    }

    // ── 存在性检查 ────────────────────────────────────────────
    async fn exists_by_code(&mut self, code: &PositionCode) -> Result<bool, PortError> {
        let result = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM org_position WHERE code = $1 AND deleted_at IS NULL) as "exists!""#,
            code.as_str()
        )
        .fetch_one(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(result)
    }

    async fn exists_by_name(&mut self, name: &PositionName) -> Result<bool, PortError> {
        let result = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM org_position WHERE name = $1 AND deleted_at IS NULL) as "exists!""#,
            name.as_str()
        )
        .fetch_one(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(result)
    }
}
