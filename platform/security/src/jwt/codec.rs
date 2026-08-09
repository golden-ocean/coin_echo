//! JWT 编解码。

use std::sync::Arc;

use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use platform_kernel::time::Clock;

use crate::jwt::config::JwtConfig;
use crate::jwt::error::JwtError;

/// 令牌声明（payload）。
///
/// `sub` 存放主体标识的字符串表示（如 `user_id.to_string()`），而非某个
/// 具体领域的 `Id<T>` 本身——JWT 是跨领域、跨服务传输的载体，本 crate
/// 不应该认识任何业务实体标签类型，那会让 `platform-security` 反过来
/// 依赖业务 crate。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claims {
    /// 主体标识（通常是用户 ID 的字符串形式）。
    pub sub: String,
    /// 签发者。
    pub iss: String,
    /// 签发时间（Unix 秒）。
    pub iat: i64,
    /// 过期时间（Unix 秒）。
    pub exp: i64,
}

/// 一次登录签发的令牌对。
#[derive(Debug, Clone, Serialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
}

/// JWT 编解码器。纯逻辑，不依赖任何 HTTP/tower 类型——`middleware::jwt`
/// 只是从请求头取出 token 字符串后调用这里的 `verify_access`。
///
/// `clock` 通过依赖注入传入而非直接调用 `Utc::now()`：测试中注入
/// [`platform_kernel::time::FixedClock`] 才能稳定断言过期逻辑，
/// 不必真的等待令牌过期。
pub struct JwtCodec {
    access_encoding: EncodingKey,
    access_decoding: DecodingKey,
    refresh_encoding: EncodingKey,
    refresh_decoding: DecodingKey,
    issuer: String,
    access_ttl: chrono::Duration,
    refresh_ttl: chrono::Duration,
    clock: Arc<dyn Clock>,
}

impl std::fmt::Debug for JwtCodec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 不打印任何密钥材料，避免密钥意外写进日志。
        f.debug_struct("JwtCodec")
            .field("issuer", &self.issuer)
            .field("access_ttl", &self.access_ttl)
            .field("refresh_ttl", &self.refresh_ttl)
            .finish_non_exhaustive()
    }
}

impl JwtCodec {
    /// 由配置与时钟构造。调用前应先对 `config` 调用过 [`JwtConfig::validate`]。
    #[must_use]
    pub fn new(config: &JwtConfig, clock: Arc<dyn Clock>) -> Self {
        Self {
            access_encoding: EncodingKey::from_secret(config.access_secret.as_bytes()),
            access_decoding: DecodingKey::from_secret(config.access_secret.as_bytes()),
            refresh_encoding: EncodingKey::from_secret(config.refresh_secret.as_bytes()),
            refresh_decoding: DecodingKey::from_secret(config.refresh_secret.as_bytes()),
            issuer: config.issuer.clone(),
            access_ttl: chrono::Duration::minutes(config.access_expire_minutes),
            refresh_ttl: chrono::Duration::hours(config.refresh_expire_hours),
            clock,
        }
    }

    /// 签发一对 access/refresh 令牌，主体为 `subject`。
    pub fn issue(&self, subject: &str) -> Result<TokenPair, JwtError> {
        let now = self.clock.now();
        let access_token = self.encode(subject, now, self.access_ttl, &self.access_encoding)?;
        let refresh_token = self.encode(subject, now, self.refresh_ttl, &self.refresh_encoding)?;
        Ok(TokenPair {
            access_token,
            refresh_token,
        })
    }

    /// 校验 access token，返回其声明。
    pub fn verify_access(&self, token: &str) -> Result<Claims, JwtError> {
        self.decode(token, &self.access_decoding)
    }

    /// 校验 refresh token，返回其声明。
    pub fn verify_refresh(&self, token: &str) -> Result<Claims, JwtError> {
        self.decode(token, &self.refresh_decoding)
    }

    fn encode(
        &self,
        subject: &str,
        now: chrono::DateTime<chrono::Utc>,
        ttl: chrono::Duration,
        key: &EncodingKey,
    ) -> Result<String, JwtError> {
        let claims = Claims {
            sub: subject.to_string(),
            iss: self.issuer.clone(),
            iat: now.timestamp(),
            exp: (now + ttl).timestamp(),
        };
        encode(&Header::default(), &claims, key).map_err(|_| JwtError::EncodingFailed)
    }

    fn decode(&self, token: &str, key: &DecodingKey) -> Result<Claims, JwtError> {
        let mut validation = Validation::default();
        validation.set_issuer(&[&self.issuer]);
        decode::<Claims>(token, key, &validation)
            .map(|data| data.claims)
            .map_err(JwtError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_kernel::time::{FixedClock, SystemClock};

    fn config() -> JwtConfig {
        JwtConfig {
            access_secret: "a".repeat(32),
            access_expire_minutes: 15,
            refresh_secret: "b".repeat(32),
            refresh_expire_hours: 720,
            issuer: "app".to_string(),
        }
    }

    #[test]
    fn access_token_round_trips_with_expected_subject_and_issuer() {
        let codec = JwtCodec::new(&config(), Arc::new(SystemClock));
        let pair = codec.issue("user-123").unwrap();
        let claims = codec.verify_access(&pair.access_token).unwrap();
        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.iss, "app");
        assert!(claims.iat <= claims.exp);
    }

    #[test]
    fn refresh_token_round_trips() {
        let codec = JwtCodec::new(&config(), Arc::new(SystemClock));
        let pair = codec.issue("user-123").unwrap();
        let claims = codec.verify_refresh(&pair.refresh_token).unwrap();
        assert_eq!(claims.sub, "user-123");
    }

    #[test]
    fn access_token_rejected_when_verified_as_refresh_token() {
        let codec = JwtCodec::new(&config(), Arc::new(SystemClock));
        let pair = codec.issue("user-123").unwrap();
        let result = codec.verify_refresh(&pair.access_token);
        assert!(matches!(result, Err(JwtError::InvalidToken)));
    }

    #[test]
    fn expired_access_token_is_rejected() {
        let clock = Arc::new(FixedClock::new(chrono::DateTime::<chrono::Utc>::UNIX_EPOCH));
        let codec = JwtCodec::new(&config(), clock);
        let pair = codec.issue("user-123").unwrap();
        let result = codec.verify_access(&pair.access_token);
        assert!(matches!(result, Err(JwtError::Expired)));
    }

    #[test]
    fn issuer_mismatch_is_rejected() {
        let mut other_issuer_config = config();
        other_issuer_config.issuer = "other-app".to_string();

        let issuing_codec = JwtCodec::new(&other_issuer_config, Arc::new(SystemClock));
        let pair = issuing_codec.issue("user-123").unwrap();

        let verifying_codec = JwtCodec::new(&config(), Arc::new(SystemClock));
        let result = verifying_codec.verify_access(&pair.access_token);
        assert!(matches!(result, Err(JwtError::InvalidClaims)));
    }

    #[test]
    fn garbage_token_is_rejected_as_invalid() {
        let codec = JwtCodec::new(&config(), Arc::new(SystemClock));
        let result = codec.verify_access("not-a-real-jwt");
        assert!(matches!(result, Err(JwtError::InvalidToken)));
    }

    #[test]
    fn debug_output_never_contains_secret_material() {
        let codec = JwtCodec::new(&config(), Arc::new(SystemClock));
        let debug_str = format!("{codec:?}");
        assert!(!debug_str.contains(&config().access_secret));
        assert!(!debug_str.contains(&config().refresh_secret));
    }
}
