//! JWT 配置。
//!
//! 对应环境变量前缀 `JWT_`。access/refresh 使用独立密钥——即便 access
//! 密钥意外泄露（例如出现在日志里），攻击者也无法用它签出长期有效的
//! refresh token。

use platform_config::ConfigMeta;
use serde::Deserialize;

/// JWT 签发配置。
#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    /// access token 签名密钥。
    pub access_secret: String,
    /// access token 有效期（分钟）。
    #[serde(default = "JwtConfig::default_access_expire_minutes")]
    pub access_expire_minutes: i64,

    /// refresh token 签名密钥，必须与 `access_secret` 不同。
    pub refresh_secret: String,
    /// refresh token 有效期（小时）。
    #[serde(default = "JwtConfig::default_refresh_expire_hours")]
    pub refresh_expire_hours: i64,

    /// 签发者（`iss` 声明），用于校验令牌确实来自本系统签发。
    #[serde(default = "JwtConfig::default_issuer")]
    pub issuer: String,
}

/// 配置语义层面的非法状态：字段本身能解析成功，但不满足安全约束。
#[derive(Debug, thiserror::Error)]
pub enum JwtConfigError {
    #[error("access_secret 长度过短（{len} 字节），至少需要 {min} 字节")]
    AccessSecretTooShort { len: usize, min: usize },

    #[error("refresh_secret 长度过短（{len} 字节），至少需要 {min} 字节")]
    RefreshSecretTooShort { len: usize, min: usize },

    #[error("access_secret 与 refresh_secret 不能相同")]
    SecretsMustDiffer,

    #[error("access_expire_minutes 必须为正数，当前为 {0}")]
    NonPositiveAccessExpiry(i64),

    #[error("refresh_expire_hours 必须为正数，当前为 {0}")]
    NonPositiveRefreshExpiry(i64),
}

impl JwtConfig {
    /// 密钥最小长度（字节）。HMAC-SHA256 建议密钥长度不小于哈希输出长度
    /// （32 字节），短密钥会削弱签名的抗爆破强度。
    const MIN_SECRET_LEN: usize = 32;

    const fn default_access_expire_minutes() -> i64 {
        15
    }

    const fn default_refresh_expire_hours() -> i64 {
        720 // 30 天
    }

    fn default_issuer() -> String {
        "app".to_string()
    }
}

impl ConfigMeta for JwtConfig {
    type Error = JwtConfigError;

    fn prefix() -> &'static str {
        "JWT_"
    }

    /// 拒绝弱密钥、拒绝双密钥相同、拒绝非法有效期。由 [`ConfigMeta::load`]
    /// 在加载后自动调用一次，失败即终止启动，不带着不安全配置对外提供
    /// 服务。
    fn validate(&self) -> Result<(), Self::Error> {
        if self.access_secret.len() < Self::MIN_SECRET_LEN {
            return Err(JwtConfigError::AccessSecretTooShort {
                len: self.access_secret.len(),
                min: Self::MIN_SECRET_LEN,
            });
        }
        if self.refresh_secret.len() < Self::MIN_SECRET_LEN {
            return Err(JwtConfigError::RefreshSecretTooShort {
                len: self.refresh_secret.len(),
                min: Self::MIN_SECRET_LEN,
            });
        }
        if self.access_secret == self.refresh_secret {
            return Err(JwtConfigError::SecretsMustDiffer);
        }
        if self.access_expire_minutes <= 0 {
            return Err(JwtConfigError::NonPositiveAccessExpiry(
                self.access_expire_minutes,
            ));
        }
        if self.refresh_expire_hours <= 0 {
            return Err(JwtConfigError::NonPositiveRefreshExpiry(
                self.refresh_expire_hours,
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_config() -> JwtConfig {
        JwtConfig {
            access_secret: "a".repeat(32),
            access_expire_minutes: 15,
            refresh_secret: "b".repeat(32),
            refresh_expire_hours: 720,
            issuer: "app".to_string(),
        }
    }

    // ---- validate() 语义校验，直接调用 ConfigMeta::validate ----

    #[test]
    fn valid_config_passes_validation() {
        assert!(valid_config().validate().is_ok());
    }

    #[test]
    fn short_access_secret_rejected() {
        let cfg = JwtConfig {
            access_secret: "short".to_string(),
            ..valid_config()
        };
        assert!(matches!(
            cfg.validate(),
            Err(JwtConfigError::AccessSecretTooShort { .. })
        ));
    }

    #[test]
    fn short_refresh_secret_rejected() {
        let cfg = JwtConfig {
            refresh_secret: "short".to_string(),
            ..valid_config()
        };
        assert!(matches!(
            cfg.validate(),
            Err(JwtConfigError::RefreshSecretTooShort { .. })
        ));
    }

    #[test]
    fn identical_secrets_rejected() {
        let same = "c".repeat(32);
        let cfg = JwtConfig {
            access_secret: same.clone(),
            refresh_secret: same,
            ..valid_config()
        };
        assert!(matches!(
            cfg.validate(),
            Err(JwtConfigError::SecretsMustDiffer)
        ));
    }

    #[test]
    fn zero_access_expiry_rejected() {
        let cfg = JwtConfig {
            access_expire_minutes: 0,
            ..valid_config()
        };
        assert!(matches!(
            cfg.validate(),
            Err(JwtConfigError::NonPositiveAccessExpiry(0))
        ));
    }

    #[test]
    fn negative_refresh_expiry_rejected() {
        let cfg = JwtConfig {
            refresh_expire_hours: -1,
            ..valid_config()
        };
        assert!(matches!(
            cfg.validate(),
            Err(JwtConfigError::NonPositiveRefreshExpiry(-1))
        ));
    }

    // ---- ConfigMeta::load_from：真正调用 trait 提供的加载+校验一体流程 ----

    #[test]
    fn load_from_applies_defaults_when_optional_vars_absent() {
        let vars = vec![
            ("JWT_ACCESS_SECRET".to_string(), "a".repeat(32)),
            ("JWT_REFRESH_SECRET".to_string(), "b".repeat(32)),
        ];
        let cfg = JwtConfig::load_from(vars).unwrap();
        assert_eq!(cfg.access_expire_minutes, 15);
        assert_eq!(cfg.refresh_expire_hours, 720);
        assert_eq!(cfg.issuer, "app");
    }

    #[test]
    fn load_from_fails_when_required_secret_missing() {
        let vars = vec![("JWT_ACCESS_SECRET".to_string(), "a".repeat(32))];
        let result = JwtConfig::load_from(vars);
        assert!(matches!(
            result,
            Err(platform_config::ConfigError::Load { .. })
        ));
    }

    /// 关键回归测试：验证 `load_from` 不仅反序列化成功，还真的调用了
    /// `validate()`——若 trait 默认实现里漏掉了校验这一步（比如复制粘贴
    /// 时被误删），这条测试能捕获到，而不是像之前 `error_message_
    /// includes_prefix_for_diagnosis` 那样绕开被测函数本身。
    #[test]
    fn load_from_rejects_semantically_invalid_config_even_if_deserializable() {
        let vars = vec![
            ("JWT_ACCESS_SECRET".to_string(), "short".to_string()), // 反序列化成功，但语义非法
            ("JWT_REFRESH_SECRET".to_string(), "b".repeat(32)),
        ];
        let result = JwtConfig::load_from(vars);
        assert!(matches!(
            result,
            Err(platform_config::ConfigError::Validation { .. })
        ));
    }

    #[test]
    fn prefix_is_jwt() {
        assert_eq!(JwtConfig::prefix(), "JWT_");
    }
}

