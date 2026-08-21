use sqlx::{PgPool, Postgres, Transaction};
use sys_application::ports::{
    DictionaryItemRepository, DictionaryRepository, UnitOfWork, UnitOfWorkError, UnitOfWorkFactory,
};

use crate::persistence::postgres::{PgDictionaryItemRepository, PgDictionaryRepository};

pub struct PgUnitOfWorkFactory {
    pool: PgPool,
}

impl PgUnitOfWorkFactory {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl UnitOfWorkFactory for PgUnitOfWorkFactory {
    async fn begin(&self) -> Result<Box<dyn UnitOfWork>, UnitOfWorkError> {
        let tx = self
            .pool
            .begin()
            .await
            .map_err(|_| UnitOfWorkError::Begin)?;
        Ok(Box::new(PgUnitOfWork { tx: Some(tx) }))
    }
}

pub struct PgUnitOfWork {
    tx: Option<Transaction<'static, Postgres>>,
}

#[async_trait::async_trait]
impl UnitOfWork for PgUnitOfWork {
    fn dict_repo<'a>(&'a mut self) -> Result<Box<dyn DictionaryRepository + 'a>, UnitOfWorkError> {
        let tx = self.tx.as_mut().ok_or(UnitOfWorkError::TransactionClosed)?;
        Ok(Box::new(PgDictionaryRepository::new(tx)))
    }

    fn dict_item_repo<'a>(
        &'a mut self,
    ) -> Result<Box<dyn DictionaryItemRepository + 'a>, UnitOfWorkError> {
        let tx = self.tx.as_mut().ok_or(UnitOfWorkError::TransactionClosed)?;
        Ok(Box::new(PgDictionaryItemRepository::new(tx)))
    }

    async fn commit(mut self: Box<Self>) -> Result<(), UnitOfWorkError> {
        let tx = self.tx.take().ok_or(UnitOfWorkError::TransactionClosed)?;
        tx.commit().await.map_err(|_| UnitOfWorkError::Commit)
    }

    async fn rollback(mut self: Box<Self>) -> Result<(), UnitOfWorkError> {
        let tx = self.tx.take().ok_or(UnitOfWorkError::TransactionClosed)?;
        tx.rollback().await.map_err(|_| UnitOfWorkError::Rollback)
    }
}

impl Drop for PgUnitOfWork {
    fn drop(&mut self) {
        if self.tx.is_some() {
            tracing::warn!(
                component = "PgUnitOfWork",
                event = "implicit_drop",
                error_code = "TRANSACTION_UNCOMMITTED",
                "Unit of work dropped without explicit commit or rollback"
            );
            // sqlx::Transaction::drop 会自动 rollback，无需手动处理
        }
    }
}
