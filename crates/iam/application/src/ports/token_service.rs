use chrono::{DateTime, Utc};

use iam_domain::id::UserId;
use platform_kernel::error::{ErrorKind, ErrorMeta};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TokenServiceError {
    #[error("令牌签发失败")]
    Issue,
    #[error("令牌无效或已损坏")]
    Invalid,
    #[error("令牌已过期")]
    Expired,
}

impl ErrorMeta for TokenServiceError {
    fn kind(&self) -> ErrorKind {
        match self {
            Self::Issue => ErrorKind::Internal,
            Self::Invalid | Self::Expired => ErrorKind::Unauthenticated,
        }
    }
    fn code(&self) -> &'static str {
        match self {
            Self::Issue => "iam.token.issue_failed",
            Self::Invalid => "iam.token.invalid",
            Self::Expired => "iam.token.expired",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub access_expires_at: DateTime<Utc>,
    pub refresh_expires_at: DateTime<Utc>,
}

/// IAM 应用层的令牌端口。只认识 `UserId`，不认识 JWT/jsonwebtoken 的任何细节
/// （不认识 Claims、不认识密钥、不认识 issuer）。
///
/// 刻意不携带角色信息：角色应该在每次鉴权时实时查询（经由 UserRoleRepository
/// 或未来接入的 Casbin），而不是固化进这个可能存活数天的 refresh token /
/// 十几分钟的 access token 里——固化进去的话，管理员改了这个人的角色，
/// 要等 token 过期才能生效，安全性和体验都更差。
///
/// 具体实现（infrastructure 层）内部包一层 `platform-security` 的
/// `JwtCodec`，负责 `UserId <-> String` 的转换。
pub trait TokenService: Send + Sync {
    /// 登录成功后签发一对 access/refresh token
    fn issue_token_pair(&self, user_id: UserId) -> Result<TokenPair, TokenServiceError>;

    /// 校验 access token，返回其中携带的用户 ID（供 AuthExtractor 中间件调用）
    fn verify_access_token(&self, token: &str) -> Result<UserId, TokenServiceError>;

    /// 校验 refresh token，返回用户 ID
    fn verify_refresh_token(&self, token: &str) -> Result<UserId, TokenServiceError>;
}
