use std::fmt;
use std::str::FromStr;
use std::sync::LazyLock;

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};
use regex::Regex;

const MAX_LEN: usize = 254;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EmailError {
    #[error("邮箱不能为空")]
    Empty,
    #[error("邮箱格式无效: {value}")]
    Invalid { value: String },
    #[error("邮箱长度不能超过 {MAX_LEN} 个字符")]
    TooLong,
}

impl ErrorMeta for EmailError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Validation
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Empty => "iam.user.email.empty",
            Self::Invalid { .. } => "iam.user.email.invalid",
            Self::TooLong => "iam.user.email.too_long",
        }
    }

    fn detail(&self) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            // Invalid 变体的 value 已经是用户输入的邮箱格式，不做二次脱敏——
            // 这是校验失败场景，值本身格式不对，不代表它是一个"真实存在的
            // 邮箱"，暴露给客户端本就是为了让用户看清自己输错了什么。
            Self::Invalid { value } => Some(format!("邮箱格式无效: {value}").into()),
            Self::Empty | Self::TooLong => None,
        }
    }

    fn fields(&self) -> Vec<FieldError> {
        let code = match self {
            Self::Empty => "required",
            Self::Invalid { .. } => "invalid_format",
            Self::TooLong => "too_long",
        };
        vec![FieldError::new("email", code)]
    }
}

static EMAIL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]{1,64}@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)+$",
    )
    .expect("EMAIL_REGEX 编译失败，正则表达式本身有误")
});

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Email(String);

impl Email {
    pub fn new(s: &str) -> Result<Self, EmailError> {
        let raw = s.trim().to_ascii_lowercase();
        if raw.is_empty() {
            return Err(EmailError::Empty);
        }
        if raw.chars().count() > MAX_LEN {
            return Err(EmailError::TooLong);
        }
        if !EMAIL_REGEX.is_match(&raw) {
            return Err(EmailError::Invalid { value: raw });
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn masked(&self) -> String {
        platform_kernel::mask::mask_email(&self.0)
    }
}

impl AsRef<str> for Email {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for Email {
    type Err = EmailError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<&str> for Email {
    type Error = EmailError;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for Email {
    type Error = EmailError;

    #[inline]
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(&value)
    }
}

impl TryFrom<&String> for Email {
    type Error = EmailError;

    #[inline]
    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.masked())
    }
}

impl fmt::Debug for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Email").field(&self.masked()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_valid_normal() {
        let value = "  TEST_USER@EXAMPLE-ORG.COM    ";
        let email = Email::new(value).unwrap();
        assert_eq!(email.as_str(), "test_user@example-org.com");
        assert!(!format!("{}", email).contains("test_user@example-org.com"));
    }

    #[test]
    fn test_email_empty_input() {
        let err1 = Email::new("").unwrap_err();
        assert_eq!(err1, EmailError::Empty);
        let err2 = Email::new("    ").unwrap_err();
        assert_eq!(err2, EmailError::Empty);
    }

    #[test]
    fn test_email_too_long() {
        let local = "a".repeat(248);
        let long_raw = format!("{}@test.com", local);
        let err = Email::new(&long_raw).unwrap_err();
        assert_eq!(err, EmailError::TooLong);
    }

    #[test]
    fn test_email_invalid_format() {
        let bad_cases = [
            "abc",
            "@test.com",
            "user@",
            "user@@test.com",
            "user@.com",
            "user@com.",
            "user name@test.com",
        ];
        for s in bad_cases {
            let err = Email::new(s).unwrap_err();
            assert_eq!(
                err,
                EmailError::Invalid {
                    value: s.to_string()
                }
            );
        }
    }

    #[test]
    fn test_from_str_parse() {
        let raw = "HELLO@DEMO.IO";
        let email = raw.parse::<Email>().unwrap();
        assert_eq!(email.as_str(), "hello@demo.io");
    }

    #[test]
    fn test_try_from_str_and_string() {
        // &str
        let email_ref: Email = "user@example.com".try_into().unwrap();
        assert_eq!(email_ref.as_str(), "user@example.com");

        // String
        let email_string: Email = String::from("ADMIN@COMPANY.ORG").try_into().unwrap();
        assert_eq!(email_string.as_str(), "admin@company.org");

        // &String
        let s = String::from("hello@world.com");
        let email_ref_string: Email = (&s).try_into().unwrap();
        assert_eq!(email_ref_string.as_str(), "hello@world.com");

        // Error path via TryFrom
        let err: Result<Email, _> = "invalid_email".try_into();
        assert_eq!(
            err.unwrap_err(),
            EmailError::Invalid {
                value: "invalid_email".to_string()
            }
        );
    }

    #[test]
    fn test_masked_no_plaintext() {
        let email = Email::new("admin123@domain.org").unwrap();
        let mask_text = email.masked();
        assert!(!mask_text.contains("admin123@domain.org"));
        assert!(mask_text.contains("***"));
    }

    #[test]
    fn test_debug_masked_output() {
        let email = Email::new("log@secret.com").unwrap();
        let debug_output = format!("{:?}", email);
        assert!(!debug_output.contains("log@secret.com"));
        assert!(debug_output.contains("***"));
    }

    // ---- ErrorMeta ----

    #[test]
    fn error_meta_kind_is_always_validation() {
        assert_eq!(EmailError::Empty.kind(), ErrorKind::Validation);
        assert_eq!(EmailError::TooLong.kind(), ErrorKind::Validation);
        assert_eq!(
            EmailError::Invalid { value: "x".into() }.kind(),
            ErrorKind::Validation
        );
    }

    #[test]
    fn error_meta_codes_are_distinct() {
        let codes = [
            EmailError::Empty.code(),
            EmailError::TooLong.code(),
            EmailError::Invalid { value: "x".into() }.code(),
        ];
        let unique: std::collections::HashSet<_> = codes.iter().collect();
        assert_eq!(unique.len(), codes.len());
        assert!(codes.iter().all(|c| c.starts_with("iam.user.email.")));
    }

    #[test]
    fn error_meta_fields_names_email_field() {
        let fields = EmailError::Empty.fields();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].field, "email");
        assert_eq!(fields[0].code, "required");
    }

    #[test]
    fn error_meta_detail_carries_invalid_value() {
        let err = EmailError::Invalid {
            value: "bad".into(),
        };
        assert_eq!(err.detail().as_deref(), Some("邮箱格式无效: bad"));
    }
}
