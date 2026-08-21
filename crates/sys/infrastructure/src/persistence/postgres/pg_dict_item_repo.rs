use sqlx::{Postgres, Transaction};

use sys_application::ports::{DictionaryItemRepository, PortError};
use sys_domain::{
    dictionary::{
        DictionaryItem,
        value_object::{DictionaryItemLabel, DictionaryItemValue},
    },
    id::{DictionaryId, DictionaryItemId},
};

use crate::persistence::models::DictionaryItemModel;

mod item_constraints {
    pub const DICT_LABEL: &str = "uk_sys_dictionary_item_dict_label";
    pub const DICT_VALUE: &str = "uk_sys_dictionary_item_dict_value";
}

pub struct PgDictionaryItemRepository<'tx, 'c> {
    tx: &'tx mut Transaction<'c, Postgres>,
}

impl<'tx, 'c> PgDictionaryItemRepository<'tx, 'c> {
    pub fn new(tx: &'tx mut Transaction<'c, Postgres>) -> Self {
        Self { tx }
    }

    fn map_sqlx_error(e: sqlx::Error) -> PortError {
        if let sqlx::Error::Database(db_err) = &e {
            if db_err.is_unique_violation() {
                return match db_err.constraint().unwrap_or_default() {
                    item_constraints::DICT_LABEL => PortError::UniqueConflict {
                        entity: "dictionary_item",
                        field: "label",
                    },
                    item_constraints::DICT_VALUE => PortError::UniqueConflict {
                        entity: "dictionary_item",
                        field: "value",
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
impl<'tx, 'c> DictionaryItemRepository for PgDictionaryItemRepository<'tx, 'c> {
    // ── 写入与修改 ──────────────────────────────────────────────
    async fn insert(&mut self, item: &DictionaryItem) -> Result<(), PortError> {
        // 假定 item.dictionary_id() 能获取其上级 DictionaryId
        let m = DictionaryItemModel::from_entity(item);

        sqlx::query!(
            r#"
                INSERT INTO sys_dictionary_item (
                    id, dictionary_id, label, value, color, is_builtin, sort, remark, status,
                    created_at, created_by, updated_at, updated_by
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9,
                    $10, $11, $12, $13
                )
            "#,
            m.id,
            m.dictionary_id,
            m.label,
            m.value,
            m.color,
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

    async fn update(&mut self, item: &DictionaryItem) -> Result<(), PortError> {
        let m = DictionaryItemModel::from_entity(item);

        let result = sqlx::query!(
            r#"
                UPDATE sys_dictionary_item SET
                    label = $2, value = $3, color = $4, sort = $5,
                    remark = $6, status = $7,
                    updated_at = $8, updated_by = $9
                WHERE id = $1
            "#,
            m.id,
            m.label,
            m.value,
            m.color,
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
                entity: "dictionary_item",
            });
        }
        Ok(())
    }

    async fn delete(&mut self, item: &DictionaryItem) -> Result<(), PortError> {
        let id = item.id().as_uuid();

        let result = sqlx::query!(
            r#"
                DELETE FROM sys_dictionary_item
                WHERE id = $1
            "#,
            id
        )
        .execute(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(PortError::NotFound {
                entity: "dictionary_item",
            });
        }
        Ok(())
    }

    // ── 实体与集合查询 ──────────────────────────────────────────
    async fn find_by_id(
        &mut self,
        id: &DictionaryItemId,
    ) -> Result<Option<DictionaryItem>, PortError> {
        let row = sqlx::query_as!(
            DictionaryItemModel,
            r#"
                SELECT
                    id, dictionary_id, label, value, color, is_builtin, sort, remark, status,
                    created_at, created_by, updated_at, updated_by
                FROM sys_dictionary_item
                WHERE id = $1
            "#,
            id.as_uuid()
        )
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        row.map(|m| DictionaryItem::try_from(&m)).transpose()
    }

    async fn find_by_dict_id_and_label(
        &mut self,
        dictionary_id: &DictionaryId,
        label: &DictionaryItemLabel,
    ) -> Result<Option<DictionaryItem>, PortError> {
        let row = sqlx::query_as!(
            DictionaryItemModel,
            r#"
                SELECT
                    id, dictionary_id, label, value, color, is_builtin, sort, remark, status,
                    created_at, created_by, updated_at, updated_by
                FROM sys_dictionary_item
                WHERE dictionary_id = $1 AND label = $2
            "#,
            dictionary_id.as_uuid(),
            label.as_str()
        )
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        row.map(|m| DictionaryItem::try_from(&m)).transpose()
    }

    async fn find_by_dict_id_and_value(
        &mut self,
        dictionary_id: &DictionaryId,
        value: &DictionaryItemValue,
    ) -> Result<Option<DictionaryItem>, PortError> {
        let row = sqlx::query_as!(
            DictionaryItemModel,
            r#"
                SELECT
                    id, dictionary_id, label, value, color, is_builtin, sort, remark, status,
                    created_at, created_by, updated_at, updated_by
                FROM sys_dictionary_item
                WHERE dictionary_id = $1 AND value = $2
            "#,
            dictionary_id.as_uuid(),
            value.as_str()
        )
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        row.map(|m| DictionaryItem::try_from(&m)).transpose()
    }

    async fn find_by_dictionary_id(
        &mut self,
        dictionary_id: &DictionaryId,
    ) -> Result<Vec<DictionaryItem>, PortError> {
        let rows = sqlx::query_as!(
            DictionaryItemModel,
            r#"
                SELECT
                    id, dictionary_id, label, value, color, is_builtin, sort, remark, status,
                    created_at, created_by, updated_at, updated_by
                FROM sys_dictionary_item
                WHERE dictionary_id = $1
                ORDER BY sort ASC
            "#,
            dictionary_id.as_uuid()
        )
        .fetch_all(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        rows.into_iter()
            .map(|m| DictionaryItem::try_from(&m))
            .collect::<Result<Vec<_>, _>>()
    }

    // ── 存在性检查 ──────────────────────────────────────────────
    // PgDictionaryItemRepository 实现里新增
    async fn exists_by_dictionary_id(
        &mut self,
        dictionary_id: &DictionaryId,
    ) -> Result<bool, PortError> {
        let result = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM sys_dictionary_item WHERE dictionary_id = $1) as "exists!""#,
            dictionary_id.as_uuid()
        )
        .fetch_one(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(result)
    }

    async fn exists_by_dict_id_and_label(
        &mut self,
        dictionary_id: &DictionaryId,
        label: &DictionaryItemLabel,
    ) -> Result<bool, PortError> {
        let result = sqlx::query_scalar!(
            r#"
                SELECT EXISTS(
                    SELECT 1 FROM sys_dictionary_item
                    WHERE dictionary_id = $1 AND label = $2
                ) as "exists!"
            "#,
            dictionary_id.as_uuid(),
            label.as_str()
        )
        .fetch_one(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(result)
    }

    async fn exists_by_dict_id_and_value(
        &mut self,
        dictionary_id: &DictionaryId,
        value: &DictionaryItemValue,
    ) -> Result<bool, PortError> {
        let result = sqlx::query_scalar!(
            r#"
                SELECT EXISTS(
                    SELECT 1 FROM sys_dictionary_item
                    WHERE dictionary_id = $1 AND value = $2
                ) as "exists!"
            "#,
            dictionary_id.as_uuid(),
            value.as_str()
        )
        .fetch_one(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(result)
    }
}
