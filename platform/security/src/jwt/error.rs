//! JWT 相关错误。

use platform_kernel::error::{ErrorKind, ErrorMeta};

/// JWT 签发/校验过程中的错误。
#[derive(Debug, thiserror::Error)]
pub enum JwtError {
    /// 令牌已过期。
    #[error("令牌已过期")]
    Expired,

    /// 令牌格式非法、签名不匹配，或结构不符合 [`super::Claims`]。
    #[error("令牌无效")]
    InvalidToken,

    /// 签发者（`iss`）等声明校验不通过。
    #[error("令牌声明校验失败")]
    InvalidClaims,

    /// 签发阶段的内部错误（如密钥编码失败），正常流程不应触发。
    #[error("令牌签发失败")]
    EncodingFailed,
}

impl ErrorMeta for JwtError {
    fn kind(&self) -> ErrorKind {
        match self {
            Self::Expired | Self::InvalidToken | Self::InvalidClaims => ErrorKind::Unauthenticated,
            Self::EncodingFailed => ErrorKind::Internal,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Expired => "security.jwt_expired",
            Self::InvalidToken => "security.jwt_invalid",
            Self::InvalidClaims => "security.jwt_claims_invalid",
            Self::EncodingFailed => "security.jwt_encoding_failed",
        }
    }
}

/// 把 `jsonwebtoken` 的底层错误归类为对外语义错误。
///
/// 归类只发生在这一处：调用方不需要、也不应该认识
/// `jsonwebtoken::errors::ErrorKind` 这个第三方类型。
impl From<jsonwebtoken::errors::Error> for JwtError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        use jsonwebtoken::errors::ErrorKind as JwtLibErrorKind;
        match err.kind() {
            JwtLibErrorKind::ExpiredSignature => Self::Expired,
            JwtLibErrorKind::InvalidIssuer
            | JwtLibErrorKind::InvalidAudience
            | JwtLibErrorKind::ImmatureSignature => Self::InvalidClaims,
            _ => Self::InvalidToken,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_failures_are_caller_fault() {
        assert_eq!(JwtError::Expired.kind(), ErrorKind::Unauthenticated);
        assert_eq!(JwtError::InvalidToken.kind(), ErrorKind::Unauthenticated);
        assert_eq!(JwtError::InvalidClaims.kind(), ErrorKind::Unauthenticated);
    }

    #[test]
    fn encoding_failure_is_internal_not_caller_fault() {
        assert_eq!(JwtError::EncodingFailed.kind(), ErrorKind::Internal);
        assert!(!JwtError::EncodingFailed.kind().is_caller_fault());
    }

    #[test]
    fn each_variant_has_stable_unique_code() {
        let codes = [
            JwtError::Expired.code(),
            JwtError::InvalidToken.code(),
            JwtError::InvalidClaims.code(),
            JwtError::EncodingFailed.code(),
        ];
        let unique: std::collections::HashSet<_> = codes.iter().collect();
        assert_eq!(unique.len(), codes.len(), "错误码不应重复");
    }

    #[test]
    fn jwt_expired_signature_maps_to_expired() {
        let err: JwtError =
            jsonwebtoken::errors::Error::from(jsonwebtoken::errors::ErrorKind::ExpiredSignature)
                .into();
        assert!(matches!(err, JwtError::Expired));
    }

    #[test]
    fn jwt_invalid_issuer_maps_to_invalid_claims() {
        let err: JwtError =
            jsonwebtoken::errors::Error::from(jsonwebtoken::errors::ErrorKind::InvalidIssuer)
                .into();
        assert!(matches!(err, JwtError::InvalidClaims));
    }

    #[test]
    fn jwt_malformed_token_maps_to_invalid_token() {
        let err: JwtError =
            jsonwebtoken::errors::Error::from(jsonwebtoken::errors::ErrorKind::InvalidToken).into();
        assert!(matches!(err, JwtError::InvalidToken));
    }
}
