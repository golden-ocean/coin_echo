use chrono::{DateTime, TimeDelta, Utc};
use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};
use zeroize::Zeroizing;

const MIN_HOURS: i64 = 24;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PasswordCredentialError {
    #[error("哈希密码格式无效: {value}")]
    Invalid { value: String },
    #[error("距上次修改不足 {MIN_HOURS} 小时，暂不能修改密码")]
    CoolingPeriodPassword,
    #[error("新密码不能与当前密码相同")]
    SameAsCurrent,
}

impl ErrorMeta for PasswordCredentialError {
    fn kind(&self) -> ErrorKind {
        // 三个变体都是"当前操作不被允许"（格式非法 / 冷却期未过 / 新旧相同），
        // 而不是服务端故障，统一归为 Validation。
        ErrorKind::Validation
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Invalid { .. } => "iam.user.password_credential.invalid",
            Self::CoolingPeriodPassword => "iam.user.password_credential.cooling_period",
            Self::SameAsCurrent => "iam.user.password_credential.same_as_current",
        }
    }

    fn detail(&self) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            Self::Invalid { .. } => Some("哈希密码格式无效".into()),
            Self::CoolingPeriodPassword => {
                Some(format!("距上次修改不足 {MIN_HOURS} 小时，暂不能修改密码").into())
            }
            Self::SameAsCurrent => Some("新密码不能与当前密码相同".into()),
        }
    }

    fn fields(&self) -> Vec<FieldError> {
        match self {
            Self::Invalid { .. } => vec![FieldError::new("password", "invalid_format")],
            Self::CoolingPeriodPassword => {
                vec![FieldError::new("password", "cooling_period")]
            }
            Self::SameAsCurrent => vec![FieldError::new("password", "same_as_current")],
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct PasswordCredential {
    hash: Zeroizing<String>,
    updated_at: DateTime<Utc>,
}

impl PasswordCredential {
    pub fn new(raw_hash: &str, updated_at: DateTime<Utc>) -> Result<Self, PasswordCredentialError> {
        Self::validate(raw_hash)?;
        Ok(Self {
            hash: Zeroizing::new(raw_hash.to_owned()),
            updated_at,
        })
    }

    fn validate(value: &str) -> Result<(), PasswordCredentialError> {
        if !value.starts_with('$') || value.split('$').count() < 3 {
            return Err(PasswordCredentialError::Invalid {
                value: value.to_owned(),
            });
        }
        Ok(())
    }

    pub fn is_expired(&self, max_days: i64, now: DateTime<Utc>) -> bool {
        let delta = now.signed_duration_since(self.updated_at);
        if delta < TimeDelta::zero() {
            return false;
        }
        delta.num_days() >= max_days
    }

    pub fn is_in_cooling_period(&self, now: DateTime<Utc>) -> bool {
        let delta = now.signed_duration_since(self.updated_at);
        if delta < TimeDelta::zero() {
            return false;
        }
        delta.num_hours() < MIN_HOURS
    }

    pub fn change(
        &self,
        new_raw_hash: &str,
        now: DateTime<Utc>,
    ) -> Result<Self, PasswordCredentialError> {
        if self.is_in_cooling_period(now) {
            return Err(PasswordCredentialError::CoolingPeriodPassword);
        }
        // 新旧 hash 相同 = 无意义修改：拒绝，避免白白 bump 版本、作废旧 token
        if new_raw_hash == self.hash.as_str() {
            return Err(PasswordCredentialError::SameAsCurrent);
        }
        Self::new(new_raw_hash, now)
    }

    pub fn reset(
        &self,
        new_raw_hash: &str,
        now: DateTime<Utc>,
    ) -> Result<Self, PasswordCredentialError> {
        Self::new(new_raw_hash, now)
    }

    pub fn hash_as_str(&self) -> &str {
        self.hash.as_str()
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

impl std::fmt::Debug for PasswordCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PasswordCredential")
            .field("hash", &platform_kernel::mask::Redacted(&self.hash))
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl AsRef<str> for PasswordCredential {
    fn as_ref(&self) -> &str {
        self.hash_as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use platform_kernel::mask::REDACT;

    fn base_time() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap()
    }

    fn valid_phc_hash() -> &'static str {
        "$argon2id$v=19,m=4096,t=3,p=2$c29tZXNhbHQ$RdescudvJCsgt3ub+b+dWRWJTmaaJObG"
    }

    #[test]
    fn test_new_valid_phc() {
        let t = base_time();
        let cred = PasswordCredential::new(valid_phc_hash(), t).unwrap();
        assert_eq!(cred.hash_as_str(), valid_phc_hash());
        assert_eq!(cred.updated_at(), t);
    }

    #[test]
    fn test_validate_invalid_phc() {
        let bad_cases = ["", "argon2id$v=19,m=4096", "$argon2id", "abc123456"];
        let t = base_time();
        for s in bad_cases {
            let err = PasswordCredential::new(s, t).unwrap_err();
            assert_eq!(
                err,
                PasswordCredentialError::Invalid {
                    value: s.to_owned()
                }
            );
        }
    }

    #[test]
    fn test_is_expired_normal() {
        let create_t = base_time();
        let cred = PasswordCredential::new(valid_phc_hash(), create_t).unwrap();
        let expire_t = create_t + TimeDelta::try_days(31).unwrap();
        assert!(cred.is_expired(30, expire_t));
        let safe_t = create_t + TimeDelta::try_days(29).unwrap();
        assert!(!cred.is_expired(30, safe_t));
    }

    #[test]
    fn test_is_expired_clock_skew() {
        let create_t = base_time();
        let cred = PasswordCredential::new(valid_phc_hash(), create_t).unwrap();
        let skew_t = create_t - TimeDelta::try_days(10).unwrap();
        assert!(!cred.is_expired(1, skew_t));
    }

    #[test]
    fn test_cooling_period_check() {
        let create_t = base_time();
        let cred = PasswordCredential::new(valid_phc_hash(), create_t).unwrap();
        let short_t = create_t + TimeDelta::try_hours(10).unwrap();
        assert!(cred.is_in_cooling_period(short_t));
        let long_t = create_t + TimeDelta::try_hours(25).unwrap();
        assert!(!cred.is_in_cooling_period(long_t));
    }

    #[test]
    fn test_change_password_cooling_limit() {
        let create_t = base_time();
        let cred = PasswordCredential::new(valid_phc_hash(), create_t).unwrap();
        let new_hash = "$argon2id$v=19,m=4096,t=3,p=2$salt2$hash2";

        let t_cool = create_t + TimeDelta::try_hours(10).unwrap();
        let err = cred.change(new_hash, t_cool).unwrap_err();
        assert_eq!(err, PasswordCredentialError::CoolingPeriodPassword);

        let t_ok = create_t + TimeDelta::try_hours(25).unwrap();
        let new_cred = cred.change(new_hash, t_ok).unwrap();
        assert_eq!(new_cred.hash_as_str(), new_hash);
        assert_eq!(new_cred.updated_at(), t_ok);
    }

    #[test]
    fn test_reset_ignore_cooling() {
        let create_t = base_time();
        let cred = PasswordCredential::new(valid_phc_hash(), create_t).unwrap();
        let new_hash = "$argon2id$v=19,m=4096,t=3,p=2$s3$h3";
        let t_soon = create_t + TimeDelta::try_hours(1).unwrap();
        let reset_cred = cred.reset(new_hash, t_soon).unwrap();
        assert_eq!(reset_cred.hash_as_str(), new_hash);
    }

    #[test]
    fn test_debug_mask_secret_hash() {
        let cred = PasswordCredential::new(valid_phc_hash(), base_time()).unwrap();
        let debug_str = format!("{:?}", cred);
        assert!(!debug_str.contains(valid_phc_hash()));
        assert!(debug_str.contains(REDACT));
    }

    #[test]
    fn test_as_ref_str() {
        let cred = PasswordCredential::new(valid_phc_hash(), base_time()).unwrap();
        fn accept_str(_s: &str) {}
        accept_str(cred.as_ref());
        assert_eq!(cred.as_ref(), cred.hash_as_str());
    }

    // ---- ErrorMeta ----

    #[test]
    fn error_meta_kind_is_validation_for_all_variants() {
        assert_eq!(
            PasswordCredentialError::Invalid { value: "x".into() }.kind(),
            ErrorKind::Validation
        );
        assert_eq!(
            PasswordCredentialError::CoolingPeriodPassword.kind(),
            ErrorKind::Validation
        );
        assert_eq!(
            PasswordCredentialError::SameAsCurrent.kind(),
            ErrorKind::Validation
        );
    }

    #[test]
    fn error_meta_detail_never_echoes_raw_hash_value() {
        // 关键安全断言：即便原始输入是"看起来像密码哈希"的字符串，
        // detail() 也绝不能把它原样带出去。
        let secret_looking_value = "$maybe$a$real$secret$hash$fragment";
        let err = PasswordCredentialError::Invalid {
            value: secret_looking_value.to_string(),
        };
        let detail = err.detail().unwrap();
        assert!(!detail.contains(secret_looking_value));
    }

    #[test]
    fn error_meta_codes_are_distinct() {
        let codes = [
            PasswordCredentialError::Invalid { value: "x".into() }.code(),
            PasswordCredentialError::CoolingPeriodPassword.code(),
            PasswordCredentialError::SameAsCurrent.code(),
        ];
        let unique: std::collections::HashSet<_> = codes.iter().collect();
        assert_eq!(unique.len(), codes.len());
        assert!(
            codes
                .iter()
                .all(|c| c.starts_with("iam.user.password_credential."))
        );
    }

    #[test]
    fn error_meta_same_as_current_fields() {
        let fields = PasswordCredentialError::SameAsCurrent.fields();
        assert_eq!(fields[0].field, "password");
        assert_eq!(fields[0].code, "same_as_current");
    }

    #[test]
    fn error_meta_fields_names_password_field() {
        let fields = PasswordCredentialError::CoolingPeriodPassword.fields();
        assert_eq!(fields[0].field, "password");
        assert_eq!(fields[0].code, "cooling_period");
    }

    #[test]
    fn test_change_same_hash_rejected() {
        let create_t = base_time();
        let cred = PasswordCredential::new(valid_phc_hash(), create_t).unwrap();

        // 冷却期已过，但改成相同 hash 仍被拒绝
        let t_after = create_t + TimeDelta::try_hours(25).unwrap();
        let err = cred.change(valid_phc_hash(), t_after).unwrap_err();
        assert_eq!(err, PasswordCredentialError::SameAsCurrent);
    }
}
