use sqlx::{Postgres, Transaction};

use crate::persistence::models::OrganizationModel;
use org_application::ports::{OrganizationRepository, PortError};
use org_domain::{
    id::OrganizationId,
    organization::{
        Organization,
        value_object::{OrganizationCode, OrganizationName},
    },
};

mod constraints {
    pub const CODE: &str = "uk_org_organization_code";
    pub const NAME: &str = "uk_org_organization_name";
}

/// OrganizationRepository 的 PostgreSQL 基础设施层具体实现
pub struct PgOrganizationRepository<'tx, 'c> {
    tx: &'tx mut Transaction<'c, Postgres>,
}

impl<'tx, 'c> PgOrganizationRepository<'tx, 'c> {
    pub fn new(tx: &'tx mut Transaction<'c, Postgres>) -> Self {
        Self { tx }
    }

    fn map_sqlx_error(e: sqlx::Error) -> PortError {
        if let sqlx::Error::Database(db_err) = &e {
            if db_err.is_unique_violation() {
                return match db_err.constraint().unwrap_or_default() {
                    constraints::CODE => PortError::UniqueConflict {
                        entity: "organization",
                        field: "code",
                    },
                    constraints::NAME => PortError::UniqueConflict {
                        entity: "organization",
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
impl<'tx, 'c> OrganizationRepository for PgOrganizationRepository<'tx, 'c> {
    // ── 写入操作 ────────────────────────────────────────────────
    async fn insert(&mut self, organization: &Organization) -> Result<(), PortError> {
        let m = OrganizationModel::from(organization);

        sqlx::query!(
            r#"
                INSERT INTO org_organization (
                    id, parent_id, name, code, contact, phone, email, sort, remark, status,
                    created_at, created_by, updated_at, updated_by
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                    $11, $12, $13, $14
                )
            "#,
            m.id,
            m.parent_id,
            m.name,
            m.code,
            m.contact,
            m.phone,
            m.email,
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

    async fn update(&mut self, organization: &Organization) -> Result<(), PortError> {
        let m = OrganizationModel::from(organization);

        let result = sqlx::query!(
            r#"
                UPDATE org_organization SET
                    parent_id = $2, name = $3, code = $4, contact = $5, phone = $6, email = $7,
                    sort = $8, remark = $9, status = $10,
                    updated_at = $11, updated_by = $12
                WHERE id = $1 AND deleted_at IS NULL
            "#,
            m.id,
            m.parent_id,
            m.name,
            m.code,
            m.contact,
            m.phone,
            m.email,
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
            return Err(PortError::NotFound {
                entity: "organization",
            });
        }
        Ok(())
    }

    async fn soft_delete(&mut self, organization: &Organization) -> Result<(), PortError> {
        let m = OrganizationModel::from(organization);

        let result = sqlx::query!(
            r#"
                UPDATE org_organization SET
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
                entity: "organization",
            });
        }
        Ok(())
    }

    // ── 聚合根查询 ────────────────────────────────────────────
    async fn find_by_id(&mut self, id: &OrganizationId) -> Result<Option<Organization>, PortError> {
        let row = sqlx::query_as!(
            OrganizationModel,
            r#"
                SELECT
                    id, parent_id, name, code, contact, phone, email, sort, remark, status,
                    created_at, created_by, updated_at, updated_by, deleted_at, deleted_by
                FROM org_organization
                WHERE id = $1 AND deleted_at IS NULL
            "#,
            id.as_uuid()
        )
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_code(
        &mut self,
        code: &OrganizationCode,
    ) -> Result<Option<Organization>, PortError> {
        let row = sqlx::query_as!(
            OrganizationModel,
            r#"
                SELECT
                    id, parent_id, name, code, contact, phone, email, sort, remark, status,
                    created_at, created_by, updated_at, updated_by, deleted_at, deleted_by
                FROM org_organization
                WHERE code = $1 AND deleted_at IS NULL
            "#,
            code.as_str()
        )
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_name(
        &mut self,
        name: &OrganizationName,
    ) -> Result<Option<Organization>, PortError> {
        let row = sqlx::query_as!(
            OrganizationModel,
            r#"
                SELECT
                    id, parent_id, name, code, contact, phone, email, sort, remark, status,
                    created_at, created_by, updated_at, updated_by, deleted_at, deleted_by
                FROM org_organization
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
    async fn exists_by_code(&mut self, code: &OrganizationCode) -> Result<bool, PortError> {
        let result = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM org_organization WHERE code = $1 AND deleted_at IS NULL) as "exists!""#,
            code.as_str()
        )
        .fetch_one(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(result)
    }

    async fn exists_by_name(&mut self, name: &OrganizationName) -> Result<bool, PortError> {
        let result = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM org_organization WHERE name = $1 AND deleted_at IS NULL) as "exists!""#,
            name.as_str()
        )
        .fetch_one(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(result)
    }

    async fn exists_children(&mut self, parent_id: &OrganizationId) -> Result<bool, PortError> {
        let result = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM org_organization WHERE parent_id = $1 AND deleted_at IS NULL) as "exists!""#,
            parent_id.as_uuid()
        )
        .fetch_one(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(result)
    }
}
