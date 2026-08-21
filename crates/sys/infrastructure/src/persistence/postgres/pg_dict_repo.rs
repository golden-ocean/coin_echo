use sqlx::{Postgres, Transaction};

use sys_application::ports::{DictionaryRepository, PortError};
use sys_domain::{
    dictionary::{
        Dictionary,
        value_object::{DictionaryCode, DictionaryName},
    },
    id::DictionaryId,
};

use crate::persistence::models::DictionaryModel;

mod dictionary_constraints {
    pub const CODE: &str = "uk_sys_dictionary_code";
    pub const NAME: &str = "uk_sys_dictionary_name";
}

pub struct PgDictionaryRepository<'tx, 'c> {
    tx: &'tx mut Transaction<'c, Postgres>,
}

impl<'tx, 'c> PgDictionaryRepository<'tx, 'c> {
    pub fn new(tx: &'tx mut Transaction<'c, Postgres>) -> Self {
        Self { tx }
    }

    fn map_sqlx_error(e: sqlx::Error) -> PortError {
        if let sqlx::Error::Database(db_err) = &e {
            if db_err.is_unique_violation() {
                return match db_err.constraint().unwrap_or_default() {
                    dictionary_constraints::CODE => PortError::UniqueConflict {
                        entity: "dictionary",
                        field: "code",
                    },
                    dictionary_constraints::NAME => PortError::UniqueConflict {
                        entity: "dictionary",
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
impl<'tx, 'c> DictionaryRepository for PgDictionaryRepository<'tx, 'c> {
    // ── 写入与修改 ──────────────────────────────────────────────
    async fn insert(&mut self, dictionary: &Dictionary) -> Result<(), PortError> {
        let m = DictionaryModel::from(dictionary);

        sqlx::query!(
            r#"
                INSERT INTO sys_dictionary (
                    id, name, code, is_builtin, sort, remark, status,
                    created_at, created_by, updated_at, updated_by
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7,
                    $8, $9, $10, $11
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
        )
        .execute(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(())
    }

    async fn update(&mut self, dictionary: &Dictionary) -> Result<(), PortError> {
        let m = DictionaryModel::from(dictionary);

        let result = sqlx::query!(
            r#"
                UPDATE sys_dictionary SET
                    name = $2, code = $3, sort = $4,
                    remark = $5, status = $6,
                    updated_at = $7, updated_by = $8
                WHERE id = $1
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
            return Err(PortError::NotFound {
                entity: "dictionary",
            });
        }
        Ok(())
    }

    async fn delete(&mut self, dictionary: &Dictionary) -> Result<(), PortError> {
        let id = dictionary.id().as_uuid();

        let result = sqlx::query!(
            r#"
                DELETE FROM sys_dictionary
                WHERE id = $1
            "#,
            id
        )
        .execute(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(PortError::NotFound {
                entity: "dictionary",
            });
        }
        Ok(())
    }

    // ── 聚合根查询 ──────────────────────────────────────────────
    async fn find_by_id(&mut self, id: &DictionaryId) -> Result<Option<Dictionary>, PortError> {
        let row = sqlx::query_as!(
            DictionaryModel,
            r#"
                SELECT
                    id, name, code, is_builtin, sort, remark, status,
                    created_at, created_by, updated_at, updated_by
                FROM sys_dictionary
                WHERE id = $1
            "#,
            id.as_uuid()
        )
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        row.map(|m| Dictionary::try_from(&m)).transpose()
    }

    async fn find_by_code(
        &mut self,
        code: &DictionaryCode,
    ) -> Result<Option<Dictionary>, PortError> {
        let row = sqlx::query_as!(
            DictionaryModel,
            r#"
                SELECT
                    id, name, code, is_builtin, sort, remark, status,
                    created_at, created_by, updated_at, updated_by
                FROM sys_dictionary
                WHERE code = $1
            "#,
            code.as_str()
        )
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        row.map(|m| Dictionary::try_from(&m)).transpose()
    }

    async fn find_by_name(
        &mut self,
        name: &DictionaryName,
    ) -> Result<Option<Dictionary>, PortError> {
        let row = sqlx::query_as!(
            DictionaryModel,
            r#"
                SELECT
                    id, name, code, is_builtin, sort, remark, status,
                    created_at, created_by, updated_at, updated_by
                FROM sys_dictionary
                WHERE name = $1
            "#,
            name.as_str()
        )
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        row.map(|m| Dictionary::try_from(&m)).transpose()
    }

    // ── 存在性检查 ──────────────────────────────────────────────
    async fn exists_by_code(&mut self, code: &DictionaryCode) -> Result<bool, PortError> {
        let result = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM sys_dictionary WHERE code = $1) as "exists!""#,
            code.as_str()
        )
        .fetch_one(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(result)
    }

    async fn exists_by_name(&mut self, name: &DictionaryName) -> Result<bool, PortError> {
        let result = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM sys_dictionary WHERE name = $1) as "exists!""#,
            name.as_str()
        )
        .fetch_one(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(result)
    }
}
