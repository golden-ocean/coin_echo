use std::sync::Arc;

use uuid::Uuid;

use iam_application::ports::{TokenPair, TokenService, TokenServiceError};
use iam_domain::id::UserId;
use platform_security::jwt::{JwtCodec, JwtError};

/// TokenService 端口在 JWT 场景下的具体实现：把 platform-security 的
/// JwtCodec（认识字符串 subject）适配成 IAM 认识的 UserId。
pub struct JwtTokenService {
    codec: Arc<JwtCodec>,
}

impl JwtTokenService {
    pub fn new(codec: Arc<JwtCodec>) -> Self {
        Self { codec }
    }
}

impl TokenService for JwtTokenService {
    fn issue_token_pair(&self, user_id: UserId) -> Result<TokenPair, TokenServiceError> {
        let subject = user_id.as_uuid().to_string();
        let pair = self.codec.issue(&subject).map_err(map_jwt_error)?;

        // 过期时间直接从刚签发出来的 token 里解出来，而不是在这里自己维护
        // 一份重复的 TTL 配置——避免适配器里的 TTL 和 JwtCodec 内部的 TTL
        // 因为改配置时漏改一处而产生不一致。
        let access_claims = self
            .codec
            .verify_access(&pair.access_token)
            .map_err(map_jwt_error)?;
        let refresh_claims = self
            .codec
            .verify_refresh(&pair.refresh_token)
            .map_err(map_jwt_error)?;

        Ok(TokenPair {
            access_token: pair.access_token,
            refresh_token: pair.refresh_token,
            access_expires_at: timestamp_to_datetime(access_claims.exp)?,
            refresh_expires_at: timestamp_to_datetime(refresh_claims.exp)?,
        })
    }

    fn verify_access_token(&self, token: &str) -> Result<UserId, TokenServiceError> {
        let claims = self.codec.verify_access(token).map_err(map_jwt_error)?;
        parse_subject(&claims.sub)
    }

    fn verify_refresh_token(&self, token: &str) -> Result<UserId, TokenServiceError> {
        let claims = self.codec.verify_refresh(token).map_err(map_jwt_error)?;
        parse_subject(&claims.sub)
    }
}

fn parse_subject(sub: &str) -> Result<UserId, TokenServiceError> {
    Uuid::parse_str(sub)
        .map(UserId::from_uuid)
        // sub 解析失败说明 token 里的内容不是一个合法的 UserId，
        // 归类为"令牌无效"而不是单独开一种错误——调用方不需要区分这种细节。
        .map_err(|_| TokenServiceError::Invalid)
}

fn timestamp_to_datetime(exp: i64) -> Result<chrono::DateTime<chrono::Utc>, TokenServiceError> {
    chrono::DateTime::from_timestamp(exp, 0).ok_or(TokenServiceError::Issue)
}

fn map_jwt_error(e: JwtError) -> TokenServiceError {
    match e {
        JwtError::Expired => TokenServiceError::Expired,
        JwtError::InvalidToken | JwtError::InvalidClaims => TokenServiceError::Invalid,
        JwtError::EncodingFailed => TokenServiceError::Issue,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use iam_domain::id::UserId;
    use platform_kernel::time::SystemClock;
    use platform_security::jwt::JwtConfig;

    fn test_config() -> JwtConfig {
        JwtConfig {
            access_secret: "a".repeat(32),
            access_expire_minutes: 15,
            refresh_secret: "b".repeat(32),
            refresh_expire_hours: 720,
            issuer: "iam-test".to_string(),
        }
    }

    fn service() -> JwtTokenService {
        let codec = JwtCodec::new(&test_config(), Arc::new(SystemClock));
        JwtTokenService::new(Arc::new(codec))
    }

    #[test]
    fn test_issue_and_verify_access_token_roundtrip() {
        let svc = service();
        let user_id = UserId::generate();

        let pair = svc.issue_token_pair(user_id).unwrap();
        let verified = svc.verify_access_token(&pair.access_token).unwrap();

        assert_eq!(verified, user_id);
        // access 过期时间应该早于 refresh 过期时间
        assert!(pair.access_expires_at < pair.refresh_expires_at);
    }

    #[test]
    fn test_refresh_token_roundtrip() {
        let svc = service();
        let user_id = UserId::generate();

        let pair = svc.issue_token_pair(user_id).unwrap();
        let verified = svc.verify_refresh_token(&pair.refresh_token).unwrap();

        assert_eq!(verified, user_id);
    }

    #[test]
    fn test_access_token_rejected_as_refresh_token() {
        // JwtCodec 用不同的密钥签 access/refresh（见 JwtConfig 文档注释），
        // 所以拿 access token 去 verify_refresh 必然验签失败。
        let svc = service();
        let user_id = UserId::generate();

        let pair = svc.issue_token_pair(user_id).unwrap();
        let result = svc.verify_refresh_token(&pair.access_token);

        assert_eq!(result.unwrap_err(), TokenServiceError::Invalid);
    }

    #[test]
    fn test_garbage_token_rejected_as_invalid() {
        let svc = service();
        let result = svc.verify_access_token("not-a-real-jwt");
        assert_eq!(result.unwrap_err(), TokenServiceError::Invalid);
    }
}
