use std::{fmt, str::FromStr, sync::LazyLock};

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};
use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StaffNoError {
    #[error("工号不能为空")]
    Empty,
    #[error("工号格式无效: {value}, 合法值格式示例：STAFF-000001")]
    Invalid { value: String },
}

impl ErrorMeta for StaffNoError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Validation
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Empty => "iam.user.staff_no.empty",
            Self::Invalid { .. } => "iam.user.staff_no.invalid",
        }
    }

    fn detail(&self) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            Self::Invalid { value } => {
                Some(format!("工号格式无效: {value}, 合法值格式示例：STAFF-000001").into())
            }
            Self::Empty => None,
        }
    }

    fn fields(&self) -> Vec<FieldError> {
        let code = match self {
            Self::Empty => "required",
            Self::Invalid { .. } => "invalid_format",
        };
        vec![FieldError::new("staff_no", code)]
    }
}

pub static STAFF_NO_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^STAFF-\d{6}$").expect("STAFF_NO_REGEX 正则表达式语法错误，请检查格式规则")
});

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct StaffNo(String);

impl StaffNo {
    pub fn new(s: &str) -> Result<Self, StaffNoError> {
        let raw = s.trim().to_string();
        if raw.is_empty() {
            return Err(StaffNoError::Empty);
        }
        if !STAFF_NO_REGEX.is_match(&raw) {
            return Err(StaffNoError::Invalid { value: raw });
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn masked(&self) -> String {
        platform_kernel::mask::Redacted::new(self.as_str()).to_string()
    }
}

impl FromStr for StaffNo {
    type Err = StaffNoError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl AsRef<str> for StaffNo {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for StaffNo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.masked())
    }
}

impl fmt::Debug for StaffNo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("StaffNo").field(&self.masked()).finish()
    }
}

impl TryFrom<&str> for StaffNo {
    type Error = StaffNoError;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for StaffNo {
    type Error = StaffNoError;

    #[inline]
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(&value)
    }
}

impl TryFrom<&String> for StaffNo {
    type Error = StaffNoError;

    #[inline]
    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::new(value.as_str())
    }
}

#[cfg(test)]
mod tests {
    use platform_kernel::mask::REDACT;

    use super::*;

    #[test]
    fn test_staff_no_valid_normal() {
        let value = "  STAFF-000123  ";
        let sn = StaffNo::new(value).unwrap();
        assert_eq!(sn.as_str(), "STAFF-000123");
        // Display 脱敏
        assert_eq!(sn.to_string(), "[REDACTED]");
    }

    #[test]
    fn test_display_masked_and_as_str_full_raw() {
        let sn = StaffNo::new("STAFF-000001").unwrap();
        // Display 脱敏
        assert_eq!(sn.to_string(), "[REDACTED]");
        // as_str 显式取明文
        assert_eq!(sn.as_str(), "STAFF-000001");
    }

    #[test]
    fn test_staff_no_empty_err() {
        let err1 = StaffNo::new("").unwrap_err();
        assert_eq!(err1, StaffNoError::Empty);
        let err2 = StaffNo::new("    ").unwrap_err();
        assert_eq!(err2, StaffNoError::Empty);
    }

    #[test]
    fn test_staff_no_invalid_format() {
        let bad_cases = [
            "staff-000123",
            "STAFF-123",
            "STAFF-1234567",
            "STAF-000123",
            "STAFF_000123",
            "123456",
            "STAFF-abcdef",
        ];
        for s in bad_cases {
            let err = StaffNo::new(s).unwrap_err();
            assert_eq!(
                err,
                StaffNoError::Invalid {
                    value: s.to_string()
                }
            );
            assert!(err.to_string().contains("合法值格式示例：STAFF-000001"));
        }
    }

    #[test]
    fn test_from_str_parse() {
        let raw = "STAFF-999999";
        let sn = raw.parse::<StaffNo>().unwrap();
        assert_eq!(sn.as_str(), raw);
    }

    #[test]
    fn test_try_from_str_and_string() {
        // &str
        let sn_ref: StaffNo = "STAFF-000123".try_into().unwrap();
        assert_eq!(sn_ref.as_str(), "STAFF-000123");

        // String
        let sn_string: StaffNo = String::from("STAFF-888888").try_into().unwrap();
        assert_eq!(sn_string.as_str(), "STAFF-888888");

        // &String
        let s = String::from("STAFF-999999");
        let sn_ref_string: StaffNo = (&s).try_into().unwrap();
        assert_eq!(sn_ref_string.as_str(), "STAFF-999999");

        // Error path via TryFrom
        let err: Result<StaffNo, _> = "STAFF-123".try_into();
        assert_eq!(
            err.unwrap_err(),
            StaffNoError::Invalid {
                value: "STAFF-123".to_string()
            }
        );
    }

    #[test]
    fn test_as_ref_str_convert() {
        let sn = StaffNo::new("STAFF-111111").unwrap();
        fn accept_str(_s: &str) {}
        accept_str(sn.as_ref());
    }

    #[test]
    fn test_masked_no_plaintext() {
        let plain = "STAFF-000666";
        let sn = StaffNo::new(plain).unwrap();
        let mask_text = sn.masked();
        assert!(!mask_text.contains(plain));
        assert_eq!(mask_text, REDACT);
    }

    #[test]
    fn test_debug_masked_output() {
        let plain = "STAFF-000888";
        let sn = StaffNo::new(plain).unwrap();
        let debug_output = format!("{:?}", sn);
        assert!(!debug_output.contains(plain));
        assert!(debug_output.contains(REDACT));
    }

    // ---- ErrorMeta ----

    #[test]
    fn error_meta_kind_is_always_validation() {
        assert_eq!(StaffNoError::Empty.kind(), ErrorKind::Validation);
        assert_eq!(
            StaffNoError::Invalid { value: "x".into() }.kind(),
            ErrorKind::Validation
        );
    }

    #[test]
    fn error_meta_codes_are_distinct() {
        let empty = StaffNoError::Empty.code();
        let invalid = StaffNoError::Invalid { value: "x".into() }.code();
        assert_ne!(empty, invalid);
        assert!(empty.starts_with("iam.user.staff_no."));
    }

    #[test]
    fn error_meta_fields_names_staff_no_field() {
        let fields = StaffNoError::Empty.fields();
        assert_eq!(fields[0].field, "staff_no");
        assert_eq!(fields[0].code, "required");
    }
}
