use iam_application::ports::{
    PermissionRepository, RolePermissionRepository, RoleRepository, UnitOfWork, UnitOfWorkError,
    UnitOfWorkFactory, UserRepository, UserRoleRepository,
};
use sqlx::{PgPool, Postgres, Transaction};

use crate::persistence::postgres::{
    PgPermissionRepository, PgRolePermissionRepository, PgRoleRepository, PgUserRepository,
    PgUserRoleRepository,
};

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
    fn user_repo<'a>(&'a mut self) -> Result<Box<dyn UserRepository + 'a>, UnitOfWorkError> {
        let tx = self.tx.as_mut().ok_or(UnitOfWorkError::TransactionClosed)?;
        Ok(Box::new(PgUserRepository::new(tx)))
    }

    fn role_repo<'a>(&'a mut self) -> Result<Box<dyn RoleRepository + 'a>, UnitOfWorkError> {
        let tx = self.tx.as_mut().ok_or(UnitOfWorkError::TransactionClosed)?;
        Ok(Box::new(PgRoleRepository::new(tx)))
    }

    fn permission_repo<'a>(
        &'a mut self,
    ) -> Result<Box<dyn PermissionRepository + 'a>, UnitOfWorkError> {
        let tx = self.tx.as_mut().ok_or(UnitOfWorkError::TransactionClosed)?;
        Ok(Box::new(PgPermissionRepository::new(tx)))
    }

    fn user_role_repo<'a>(
        &'a mut self,
    ) -> Result<Box<dyn UserRoleRepository + 'a>, UnitOfWorkError> {
        let tx = self.tx.as_mut().ok_or(UnitOfWorkError::TransactionClosed)?;
        Ok(Box::new(PgUserRoleRepository::new(tx)))
    }

    fn role_permission_repo<'a>(
        &'a mut self,
    ) -> Result<Box<dyn RolePermissionRepository + 'a>, UnitOfWorkError> {
        let tx = self.tx.as_mut().ok_or(UnitOfWorkError::TransactionClosed)?;
        Ok(Box::new(PgRolePermissionRepository::new(tx)))
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
