use std::borrow::Cow;

use crate::{
    ports::{PortError, UnitOfWorkError},
    queries::QueryError,
};

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};
use sys_domain::error::DomainError;

/// IAM 应用统一对外错误
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Port(#[from] PortError),
    #[error(transparent)]
    Query(#[from] QueryError),

    #[error(transparent)]
    UnitOfWork(#[from] UnitOfWorkError),

    // ---- App 层自身语义 ----
    #[error("参数校验失败: {0}")]
    Validation(String),
    #[error("未认证或登录已过期")]
    Unauthorized,
    #[error("无权限执行该操作")]
    Forbidden,
    #[error("服务内部异常，请稍后重试")]
    Internal,
}

impl ErrorMeta for AppError {
    fn kind(&self) -> ErrorKind {
        match self {
            // ---- 下层错误：完全委托 ----
            Self::Domain(e) => e.kind(),
            Self::Port(e) => e.kind(),
            Self::Query(e) => e.kind(),

            // 事务/工作单元错误：绝大多数是依赖不可用，事务已关闭属于编程错误
            Self::UnitOfWork(e) => match e {
                UnitOfWorkError::TransactionClosed => ErrorKind::Internal,
                _ => ErrorKind::Unavailable,
            },

            Self::Validation(_) => ErrorKind::Validation,
            Self::Unauthorized => ErrorKind::Unauthenticated,
            Self::Forbidden => ErrorKind::Forbidden,
            Self::Internal => ErrorKind::Internal,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            // ---- 下层错误：完全委托 ----
            Self::Domain(e) => e.code(),
            Self::Port(e) => e.code(),
            Self::Query(e) => e.code(),

            Self::UnitOfWork(e) => e.code(),

            Self::Validation(_) => "iam.app.validation",
            Self::Unauthorized => "iam.app.unauthorized",
            Self::Forbidden => "iam.app.forbidden",
            Self::Internal => "iam.app.internal",
        }
    }

    fn detail(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::Domain(e) => e.detail(),
            Self::Port(e) => e.detail(),
            Self::Query(e) => e.detail(),
            // 应用层校验：回显调用方输入的描述（调用方错误，安全）
            Self::Validation(msg) => Some(Cow::Borrowed(msg.as_str())),
            // 其余服务端错误（含 PasswordHasher/TokenService）一律不回显实现细节
            _ => None,
        }
    }

    fn fields(&self) -> Vec<FieldError> {
        match self {
            Self::Domain(e) => e.fields(),
            Self::Port(e) => e.fields(),
            Self::Query(e) => e.fields(),

            _ => Vec::new(),
        }
    }
}
