use std::borrow::Cow;

use crate::{
    ports::{PasswordHasherError, PortError, UnitOfWorkError},
    queries::error::QueryError,
};

use iam_domain::error::DomainError;
use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};

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

    #[error("密码哈希错误: {0}")]
    PasswordHasher(#[from] PasswordHasherError),

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

            // 密码哈希：哈希失败是服务端故障，校验失败是调用方凭证错误
            Self::PasswordHasher(e) => match e {
                PasswordHasherError::Hash => ErrorKind::Internal,
                PasswordHasherError::Verify => ErrorKind::Unauthenticated,
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
            Self::PasswordHasher(e) => e.code(),

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
            // 其余服务端错误一律不回显实现细节（SQL/内网/哈希/脏数据等）
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

#[cfg(test)]
mod tests {
    use super::*;
    use iam_domain::id::UserId;
    use platform_kernel::meta::StatusError;
    use uuid::Uuid;

    fn user_id() -> UserId {
        UserId::from_uuid(Uuid::nil())
    }

    // ---- kind 映射 ----

    #[test]
    fn domain_delegates_kind_code() {
        let err = AppError::Domain(DomainError::UserNotFound { id: user_id() });
        assert_eq!(err.kind(), ErrorKind::NotFound);
        assert_eq!(err.code(), "iam.user.not_found");
    }

    #[test]
    fn port_error_maps_to_expected_kinds() {
        assert_eq!(
            AppError::Port(PortError::NotFound { entity: "user" }).kind(),
            ErrorKind::NotFound
        );
        assert_eq!(
            AppError::Port(PortError::UniqueConflict {
                entity: "user",
                field: "email"
            })
            .kind(),
            ErrorKind::Conflict
        );
        assert_eq!(
            AppError::Port(PortError::VersionConflict { entity: "user" }).kind(),
            ErrorKind::Conflict
        );
        assert_eq!(
            AppError::Port(PortError::Database).kind(),
            ErrorKind::Unavailable
        );
        assert_eq!(
            AppError::Port(PortError::ValueConvert {
                field: "status",
                value: "open".into()
            })
            .kind(),
            ErrorKind::Internal
        );
        assert_eq!(
            AppError::Port(PortError::Infrastructure("boom".into())).kind(),
            ErrorKind::Internal
        );
    }

    #[test]
    fn query_error_maps_to_expected_kinds() {
        assert_eq!(
            AppError::Query(QueryError::NotFound).kind(),
            ErrorKind::NotFound
        );
        assert_eq!(
            AppError::Query(QueryError::InvalidParameter { reason: "x".into() }).kind(),
            ErrorKind::Validation
        );
        assert_eq!(
            AppError::Query(QueryError::Database).kind(),
            ErrorKind::Unavailable
        );
        assert_eq!(
            AppError::Query(QueryError::Timeout).kind(),
            ErrorKind::Timeout
        );
    }

    #[test]
    fn unit_of_work_maps_to_expected_kinds() {
        assert_eq!(
            AppError::UnitOfWork(UnitOfWorkError::Commit).kind(),
            ErrorKind::Unavailable
        );
        assert_eq!(
            AppError::UnitOfWork(UnitOfWorkError::TransactionClosed).kind(),
            ErrorKind::Internal
        );
    }

    #[test]
    fn password_hasher_maps_to_expected_kinds() {
        assert_eq!(
            AppError::PasswordHasher(PasswordHasherError::Hash).kind(),
            ErrorKind::Internal
        );
        assert_eq!(
            AppError::PasswordHasher(PasswordHasherError::Verify).kind(),
            ErrorKind::Unauthenticated
        );
    }

    #[test]
    fn app_level_semantics_map_to_expected_kinds() {
        assert_eq!(
            AppError::Validation("x".into()).kind(),
            ErrorKind::Validation
        );
        assert_eq!(AppError::Unauthorized.kind(), ErrorKind::Unauthenticated);
        assert_eq!(AppError::Forbidden.kind(), ErrorKind::Forbidden);
        assert_eq!(AppError::Internal.kind(), ErrorKind::Internal);
    }

    // ---- detail 安全性 ----

    #[test]
    fn validation_detail_echoes_caller_input() {
        let err = AppError::Validation("字段 x 不能为空".into());
        assert_eq!(err.detail().as_deref(), Some("字段 x 不能为空"));
    }

    #[test]
    fn internal_detail_never_exposed() {
        // 服务端错误 detail 一律 None，不泄漏 SQL/内网/哈希等实现信息
        assert!(AppError::Internal.detail().is_none());
        assert!(
            AppError::Port(PortError::Infrastructure("secret sql".into()))
                .detail()
                .is_none()
        );
        assert!(
            AppError::Port(PortError::ValueConvert {
                field: "phone",
                value: "13900139000".into()
            })
            .detail()
            .is_none()
        );
        assert!(
            AppError::PasswordHasher(PasswordHasherError::Hash)
                .detail()
                .is_none()
        );
    }

    // ---- fields 委托 ----

    #[test]
    fn domain_fields_are_delegated() {
        let err = AppError::Domain(DomainError::Status(StatusError::Empty));
        assert_eq!(err.fields().len(), 1);
        assert_eq!(err.fields()[0].field, "status");
        assert_eq!(err.fields()[0].code, "required");
    }

    // ---- retryable ----

    #[test]
    fn retryable_follows_kind_defaults() {
        assert!(AppError::Query(QueryError::Timeout).retryable());
        assert!(AppError::Port(PortError::Database).retryable());
        assert!(AppError::UnitOfWork(UnitOfWorkError::Commit).retryable());
        assert!(!AppError::PasswordHasher(PasswordHasherError::Verify).retryable());
        assert!(!AppError::Internal.retryable());
    }

    // ---- code 唯一性 ----

    #[test]
    fn codes_are_unique_and_namespaced() {
        let codes = vec![
            AppError::Domain(DomainError::UserNotFound { id: user_id() }).code(),
            AppError::Port(PortError::NotFound { entity: "user" }).code(),
            AppError::Port(PortError::UniqueConflict {
                entity: "user",
                field: "email",
            })
            .code(),
            AppError::Port(PortError::VersionConflict { entity: "user" }).code(),
            AppError::Port(PortError::StaffNoGenerateFailed).code(),
            AppError::Port(PortError::ValueConvert {
                field: "x",
                value: "y".into(),
            })
            .code(),
            AppError::Port(PortError::Database).code(),
            AppError::Port(PortError::Infrastructure("x".into())).code(),
            AppError::Query(QueryError::NotFound).code(),
            AppError::Query(QueryError::InvalidParameter { reason: "x".into() }).code(),
            AppError::Query(QueryError::Database).code(),
            AppError::Query(QueryError::Timeout).code(),
            AppError::UnitOfWork(UnitOfWorkError::Connection).code(),
            AppError::UnitOfWork(UnitOfWorkError::Begin).code(),
            AppError::UnitOfWork(UnitOfWorkError::Commit).code(),
            AppError::UnitOfWork(UnitOfWorkError::Rollback).code(),
            AppError::UnitOfWork(UnitOfWorkError::TransactionClosed).code(),
            AppError::PasswordHasher(PasswordHasherError::Hash).code(),
            AppError::PasswordHasher(PasswordHasherError::Verify).code(),
            AppError::Validation("x".into()).code(),
            AppError::Unauthorized.code(),
            AppError::Forbidden.code(),
            AppError::Internal.code(),
        ];

        let unique: std::collections::HashSet<_> = codes.iter().collect();
        assert_eq!(unique.len(), codes.len(), "错误码必须唯一");
        assert!(codes.iter().all(|c| c.starts_with("iam.")));
    }
}
