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

/// 活跃工作单元端口
#[async_trait::async_trait]
pub trait UnitOfWork: Send {
    fn user_repo<'a>(&'a mut self) -> Result<Box<dyn UserRepository + 'a>, UnitOfWorkError>;
    fn role_repo<'a>(&'a mut self) -> Result<Box<dyn RoleRepository + 'a>, UnitOfWorkError>;

    async fn commit(self: Box<Self>) -> Result<(), UnitOfWorkError>;
    async fn rollback(self: Box<Self>) -> Result<(), UnitOfWorkError>;
}

/// 工作单元管理器端口
#[async_trait::async_trait]
pub trait UnitOfWorkFactory: Send + Sync {
    async fn begin(&self) -> Result<Box<dyn UnitOfWork>, UnitOfWorkError>;
}
