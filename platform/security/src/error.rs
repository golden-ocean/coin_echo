//! crate 级聚合错误。
//!
//! 每个 feature 已经在自己的 `error.rs` 里实现了 [`ErrorMeta`]，这里只做
//! `#[from]` 委托，不重复 `match` 一遍语义分类——避免同一份"哪个错误属于
//! 什么 kind"的知识分散在两处、后续漂移。

use std::borrow::Cow;

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};

use crate::casbin::CasbinError;
use crate::context::SecurityContextError;
use crate::jwt::JwtError;
use crate::password::PasswordError;

#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error(transparent)]
    Jwt(#[from] JwtError),
    #[error(transparent)]
    Password(#[from] PasswordError),
    #[error(transparent)]
    Casbin(#[from] CasbinError),
    #[error(transparent)]
    Context(#[from] SecurityContextError),
}

impl ErrorMeta for SecurityError {
    fn kind(&self) -> ErrorKind {
        match self {
            Self::Jwt(e) => e.kind(),
            Self::Password(e) => e.kind(),
            Self::Casbin(e) => e.kind(),
            Self::Context(e) => e.kind(),
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Jwt(e) => e.code(),
            Self::Password(e) => e.code(),
            Self::Casbin(e) => e.code(),
            Self::Context(e) => e.code(),
        }
    }

    fn detail(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::Jwt(e) => e.detail(),
            Self::Password(e) => e.detail(),
            Self::Casbin(e) => e.detail(),
            Self::Context(e) => e.detail(),
        }
    }

    fn fields(&self) -> Vec<FieldError> {
        match self {
            Self::Jwt(e) => e.fields(),
            Self::Password(e) => e.fields(),
            Self::Casbin(e) => e.fields(),
            Self::Context(e) => e.fields(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delegates_kind_and_code_to_underlying_jwt_error() {
        let err: SecurityError = JwtError::Expired.into();
        assert_eq!(err.kind(), JwtError::Expired.kind());
        assert_eq!(err.code(), JwtError::Expired.code());
    }
}
