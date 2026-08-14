use std::{fmt, str::FromStr, sync::LazyLock};

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};
use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PhoneError {
    #[error("电话号码不能为空")]
    Empty,
    #[error("无效电话号码格式: {value}")]
    Invalid { value: String },
}

impl ErrorMeta for PhoneError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Validation
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Empty => "iam.user.phone.empty",
            Self::Invalid { .. } => "iam.user.phone.invalid",
        }
    }

    fn detail(&self) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            Self::Invalid { value } => Some(format!("无效电话号码格式: {value}").into()),
            Self::Empty => None,
        }
    }

    fn fields(&self) -> Vec<FieldError> {
        let code = match self {
            Self::Empty => "required",
            Self::Invalid { .. } => "invalid_format",
        };
        vec![FieldError::new("phone", code)]
    }
}

static PHONE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\+?[1-9]\d{7,14}$").expect("PHONE_REGEX 正则表达式语法错误，请检查正则文本")
});

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Phone(String);

impl Phone {
    pub fn new(s: &str) -> Result<Self, PhoneError> {
        let raw = s.trim().to_ascii_lowercase();
        if raw.is_empty() {
            return Err(PhoneError::Empty);
        }
        if !PHONE_REGEX.is_match(&raw) {
            return Err(PhoneError::Invalid { value: raw });
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn masked(&self) -> String {
        platform_kernel::mask::mask_phone(&self.0)
    }
}

impl AsRef<str> for Phone {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for Phone {
    type Err = PhoneError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl fmt::Display for Phone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.masked())
    }
}

impl fmt::Debug for Phone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Phone").field(&self.masked()).finish()
    }
}

impl TryFrom<&str> for Phone {
    type Error = PhoneError;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for Phone {
    type Error = PhoneError;

    #[inline]
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(&value)
    }
}

impl TryFrom<&String> for Phone {
    type Error = PhoneError;

    #[inline]
    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::new(value.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phone_valid_normal() {
        let input1 = " 13800138000 ";
        let phone1 = Phone::new(input1).unwrap();
        assert_eq!(phone1.as_str(), "13800138000");
        let input2 = "+12025550108";
        let phone2 = Phone::new(input2).unwrap();
        assert_eq!(phone2.as_str(), "+12025550108");
        assert!(!format!("{}", phone1).is_empty());
    }

    #[test]
    fn test_phone_empty_err() {
        let err1 = Phone::new("").unwrap_err();
        assert_eq!(err1, PhoneError::Empty);
        let err2 = Phone::new("    ").unwrap_err();
        assert_eq!(err2, PhoneError::Empty);
    }

    #[test]
    fn test_phone_invalid_format() {
        let bad_cases = [
            "013800138000",
            "123",
            "abc",
            "+013800138000",
            "13800138000000000",
        ];
        for s in bad_cases {
            let err = Phone::new(s).unwrap_err();
            assert_eq!(
                err,
                PhoneError::Invalid {
                    value: s.to_ascii_lowercase()
                }
            );
        }
    }

    #[test]
    fn test_from_str_parse() {
        let raw = "+8613900139000";
        let phone = raw.parse::<Phone>().unwrap();
        assert_eq!(phone.as_str(), raw);
    }

    #[test]
    fn test_try_from_str_and_string() {
        // &str
        let phone_ref: Phone = "13800138000".try_into().unwrap();
        assert_eq!(phone_ref.as_str(), "13800138000");

        // String
        let phone_string: Phone = String::from("+12025550108").try_into().unwrap();
        assert_eq!(phone_string.as_str(), "+12025550108");

        // &String
        let s = String::from("13900139000");
        let phone_ref_string: Phone = (&s).try_into().unwrap();
        assert_eq!(phone_ref_string.as_str(), "13900139000");

        // Error path via TryFrom
        let err: Result<Phone, _> = "invalid_phone".try_into();
        assert_eq!(
            err.unwrap_err(),
            PhoneError::Invalid {
                value: "invalid_phone".to_string()
            }
        );
    }

    #[test]
    fn test_as_ref_str() {
        let phone = Phone::new("13700137000").unwrap();
        fn take_str(_s: &str) {}
        take_str(phone.as_ref());
    }

    #[test]
    fn test_masked_no_plaintext() {
        let plain = "13800138000";
        let phone = Phone::new(plain).unwrap();
        let mask_text = phone.masked();
        assert!(!mask_text.contains(plain));
    }

    #[test]
    fn test_debug_masked_output() {
        let plain = "+12025550108";
        let phone = Phone::new(plain).unwrap();
        let debug_output = format!("{:?}", phone);
        assert!(!debug_output.contains(plain));
    }

    #[test]
    fn test_display_masked_and_as_str_full_raw() {
        let phone = Phone::new("13900139000").unwrap();
        // Display 脱敏，不泄漏明文
        assert_eq!(phone.to_string(), "139****9000");
        // as_str 显式取明文
        assert_eq!(phone.as_str(), "13900139000");
    }

    // ---- ErrorMeta ----

    #[test]
    fn error_meta_kind_is_always_validation() {
        assert_eq!(PhoneError::Empty.kind(), ErrorKind::Validation);
        assert_eq!(
            PhoneError::Invalid { value: "x".into() }.kind(),
            ErrorKind::Validation
        );
    }

    #[test]
    fn error_meta_codes_are_distinct() {
        let empty = PhoneError::Empty.code();
        let invalid = PhoneError::Invalid { value: "x".into() }.code();
        assert_ne!(empty, invalid);
        assert!(empty.starts_with("iam.user.phone."));
    }

    #[test]
    fn error_meta_fields_names_phone_field() {
        let fields = PhoneError::Empty.fields();
        assert_eq!(fields[0].field, "phone");
        assert_eq!(fields[0].code, "required");
    }
}
