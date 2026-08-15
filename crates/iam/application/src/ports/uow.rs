use std::future::Future;
use std::pin::Pin;

use platform_kernel::error::{ErrorKind, ErrorMeta};

use crate::ports::{RoleRepository, UserRepository};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UnitOfWorkError {
    #[error("获取数据库连接失败")]
    Connection,
    #[error("开启事务失败")]
    Begin,
    #[error("提交事务失败")]
    Commit,
    #[error("回滚事务失败")]
    Rollback,
    #[error("事务已关闭，无法执行操作")]
    TransactionClosed,
}

impl ErrorMeta for UnitOfWorkError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Internal
    }
    fn code(&self) -> &'static str {
        match self {
            Self::Connection => "iam.uow.connection_failed",
            Self::Commit => "iam.uow.commit_failed",
            Self::Rollback => "iam.uow.rollback_failed",
            Self::Begin => "iam.uow.begin_failed",
            Self::TransactionClosed => "iam.uow.transaction_closed",
        }
    }
}

/// 活跃工作单元端口 —— 保持 dyn-safe，只放最基础的能力
#[async_trait::async_trait]
pub trait UnitOfWork: Send {
    fn user_repo<'a>(&'a mut self) -> Result<Box<dyn UserRepository + 'a>, UnitOfWorkError>;
    fn role_repo<'a>(&'a mut self) -> Result<Box<dyn RoleRepository + 'a>, UnitOfWorkError>;
    async fn commit(self: Box<Self>) -> Result<(), UnitOfWorkError>;
    async fn rollback(self: Box<Self>) -> Result<(), UnitOfWorkError>;
}

/// 工作单元管理器端口 —— 只保留 begin，维持 dyn-safe，
/// 这样 `&dyn UnitOfWorkFactory` / `Box<dyn UnitOfWorkFactory>` 才能继续被用作依赖注入
#[async_trait::async_trait]
pub trait UnitOfWorkFactory: Send + Sync {
    async fn begin(&self) -> Result<Box<dyn UnitOfWork>, UnitOfWorkError>;
}

/// 一个 boxed、可跨 await 点使用的 Future 别名，避免到处写 Pin<Box<dyn Future...>>
pub type UowFuture<'a, T, E> = Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'a>>;

/// 因为不需要被 dyn 使用，方法可以带泛型，不受 object-safety 限制。
#[async_trait::async_trait]
pub trait UnitOfWorkFactoryExt: UnitOfWorkFactory {
    /// 在一个事务内执行闭包：
    /// - 闭包返回 Ok  -> commit
    /// - 闭包返回 Err -> rollback（rollback 本身失败不会覆盖原始业务错误）
    async fn transaction<F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: for<'a> FnOnce(&'a mut dyn UnitOfWork) -> UowFuture<'a, T, E> + Send,
        T: Send,
        E: From<UnitOfWorkError> + Send,
    {
        let mut uow = self.begin().await.map_err(E::from)?;

        match f(&mut *uow).await {
            Ok(val) => {
                uow.commit().await.map_err(E::from)?;
                Ok(val)
            }
            Err(e) => {
                // rollback 失败也不覆盖原始业务错误，但会打日志方便排查
                if let Err(rollback_err) = uow.rollback().await {
                    tracing::error!(
                        component = "UnitOfWorkFactoryExt",
                        event = "rollback_failed_after_error",
                        error = ?rollback_err,
                        "rollback failed while handling a prior error"
                    );
                }
                Err(e)
            }
        }
    }
}

// blanket impl：任何实现了 UnitOfWorkFactory 的类型（含 dyn UnitOfWorkFactory 本身）
// 自动获得 .transaction(...) 方法
impl<F: UnitOfWorkFactory + ?Sized> UnitOfWorkFactoryExt for F {}

// pub async fn execute_in_uow<F, T, E>(factory: &dyn UnitOfWorkFactory, f: F) -> Result<T, E>
// where
//     F: for<'a> FnOnce(&'a mut dyn UnitOfWork) -> BoxFuture<'a, Result<T, E>> + Send,
//     T: Send,
//     E: From<UnitOfWorkError> + Send,
// {
//     let mut uow = factory.begin().await?;

//     match f(&mut *uow).await {
//         Ok(val) => {
//             uow.commit().await?;
//             Ok(val)
//         }
//         Err(e) => {
//             let _ = uow.rollback().await; // rollback 失败不覆盖原始业务错误
//             Err(e)
//         }
//     }
// }
