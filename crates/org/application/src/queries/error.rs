use platform_kernel::error::{ErrorKind, ErrorMeta};

#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("查询目标不存在")]
    NotFound,

    #[error("查询参数不合法: {reason}")]
    InvalidParameter { reason: String },

    #[error("查询执行失败")]
    Database,

    #[error("查询超时")]
    Timeout,
}

// sqlx::Error 分类映射
impl From<sqlx::Error> for QueryError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => QueryError::NotFound,
            sqlx::Error::PoolTimedOut => QueryError::Timeout,
            _ => QueryError::Database,
        }
    }
}

impl ErrorMeta for QueryError {
    fn kind(&self) -> ErrorKind {
        match self {
            Self::NotFound => ErrorKind::NotFound,
            Self::InvalidParameter { .. } => ErrorKind::Validation,
            Self::Database => ErrorKind::Unavailable,
            Self::Timeout => ErrorKind::Timeout,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::NotFound => "org.query.not_found",
            Self::InvalidParameter { .. } => "org.query.invalid_parameter",
            Self::Database => "org.query.database_error",
            Self::Timeout => "org.query.timeout",
        }
    }

    fn detail(&self) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            Self::InvalidParameter { reason } => Some(reason.as_str().into()),
            _ => None,
        }
    }
}
