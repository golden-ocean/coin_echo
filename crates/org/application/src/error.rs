use std::borrow::Cow;

use crate::{
    ports::{PortError, UnitOfWorkError},
    queries::QueryError,
};

use org_domain::error::DomainError;
use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};

/// ORG 应用统一对外错误
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
            Self::UnitOfWork(e) => e.kind(),

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

            Self::Validation(_) => "org.app.validation",
            Self::Unauthorized => "org.app.unauthorized",
            Self::Forbidden => "org.app.forbidden",
            Self::Internal => "org.app.internal",
        }
    }

    fn detail(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::Domain(e) => e.detail(),
            Self::Port(e) => e.detail(),
            Self::Query(e) => e.detail(),
            Self::UnitOfWork(e) => e.detail(),
            // 应用层校验：回显调用方输入的描述（调用方错误，安全）
            Self::Validation(msg) => Some(Cow::Borrowed(msg.as_str())),
            // 其余服务端错误一律不回显实现细节
            _ => None,
        }
    }

    fn fields(&self) -> Vec<FieldError> {
        match self {
            Self::Domain(e) => e.fields(),
            Self::Port(e) => e.fields(),
            Self::Query(e) => e.fields(),
            Self::UnitOfWork(e) => e.fields(),
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use org_domain::id::OrganizationId;
    use uuid::Uuid;

    fn organization_id() -> OrganizationId {
        OrganizationId::from_uuid(Uuid::nil())
    }

    // ---- kind 映射 ----

    #[test]
    fn domain_delegates_kind_code() {
        let err = AppError::Domain(DomainError::OrganizationNotFound {
            id: organization_id(),
        });
        assert_eq!(err.kind(), ErrorKind::NotFound);
        assert_eq!(err.code(), "org.organization.not_found");
    }

    #[test]
    fn port_error_maps_to_expected_kinds() {
        assert_eq!(
            AppError::Port(PortError::NotFound {
                entity: "organization"
            })
            .kind(),
            ErrorKind::NotFound
        );
        assert_eq!(
            AppError::Port(PortError::UniqueConflict {
                entity: "organization",
                field: "code"
            })
            .kind(),
            ErrorKind::Conflict
        );
        assert_eq!(
            AppError::Port(PortError::HasChildren {
                entity: "organization"
            })
            .kind(),
            ErrorKind::Conflict
        );
        assert_eq!(
            AppError::Port(PortError::HasMembers { entity: "position" }).kind(),
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
    }

    // ---- fields 委托 ----

    #[test]
    fn port_fields_are_delegated() {
        let err = AppError::Port(PortError::UniqueConflict {
            entity: "organization",
            field: "name",
        });
        assert_eq!(err.fields().len(), 1);
        assert_eq!(err.fields()[0].field, "name");
        assert_eq!(err.fields()[0].code, "unique_violation");
    }

    #[test]
    fn has_members_fields_are_delegated() {
        let err = AppError::Port(PortError::HasMembers { entity: "position" });
        assert_eq!(err.fields().len(), 1);
        assert_eq!(err.fields()[0].field, "id");
        assert_eq!(err.fields()[0].code, "has_members");
    }

    // ---- retryable ----

    #[test]
    fn retryable_follows_kind_defaults() {
        assert!(AppError::Query(QueryError::Timeout).retryable());
        assert!(AppError::Port(PortError::Database).retryable());
        assert!(!AppError::Internal.retryable());
    }

    // ---- code 唯一性 ----

    #[test]
    fn codes_are_unique_and_namespaced() {
        let codes = vec![
            AppError::Domain(DomainError::OrganizationNotFound {
                id: organization_id(),
            })
            .code(),
            AppError::Port(PortError::NotFound {
                entity: "organization",
            })
            .code(),
            AppError::Port(PortError::UniqueConflict {
                entity: "organization",
                field: "code",
            })
            .code(),
            AppError::Port(PortError::HasChildren {
                entity: "organization",
            })
            .code(),
            AppError::Port(PortError::HasMembers { entity: "position" }).code(),
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
            AppError::Validation("x".into()).code(),
            AppError::Unauthorized.code(),
            AppError::Forbidden.code(),
            AppError::Internal.code(),
        ];

        let unique: std::collections::HashSet<_> = codes.iter().collect();
        assert_eq!(unique.len(), codes.len(), "错误码必须唯一");
        assert!(codes.iter().all(|c| c.starts_with("org.")));
    }
}
