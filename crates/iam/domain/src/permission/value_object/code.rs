use std::{fmt, str::FromStr};

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};

/// 权限编码校验错误枚举
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PermissionCodeError {
    #[error("权限编码不能为空")]
    Empty,
    #[error("权限编码过长")]
    TooLong,
    #[error("权限编码格式无效: {value}")]
    Invalid { value: String },
}

impl ErrorMeta for PermissionCodeError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Validation
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Empty => "iam.permission.code.empty",
            Self::TooLong => "iam.permission.code.too_long",
            Self::Invalid { .. } => "iam.permission.code.invalid",
        }
    }

    fn detail(&self) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            Self::Invalid { value } => Some(format!("权限编码格式无效: '{value}'").into()),
            _ => None,
        }
    }

    fn fields(&self) -> Vec<FieldError> {
        let code = match self {
            Self::Empty => "required",
            Self::TooLong => "too_long",
            Self::Invalid { .. } => "invalid_format",
        };
        vec![FieldError::new("code", code)]
    }
}

/// 权限标识/编码 值对象 VO
///
/// # 业务约束
/// 1. 首尾空白自动 trim
/// 2. 非空
/// 3. 最大长度128个字符
/// 4. 仅允许小写字母、数字、下划线、冒号、短横线（如: "iam:user:add"）
///
/// # 创建方式
/// - PermissionCode::new(&str) -> Result<Self, PermissionCodeError>
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PermissionCode(String);

impl PermissionCode {
    const MAX_LEN: usize = 128;

    pub fn new(s: impl Into<String>) -> Result<Self, PermissionCodeError> {
        let raw = s.into();
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(PermissionCodeError::Empty);
        }
        if trimmed.chars().count() > Self::MAX_LEN {
            return Err(PermissionCodeError::TooLong);
        }
        let valid = trimmed
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | ':' | '-'));
        if !valid {
            return Err(PermissionCodeError::Invalid {
                value: trimmed.to_string(),
            });
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PermissionCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PermissionCode {
    type Err = PermissionCodeError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<&str> for PermissionCode {
    type Error = PermissionCodeError;

    #[inline]
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl TryFrom<String> for PermissionCode {
    type Error = PermissionCodeError;

    #[inline]
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(&s)
    }
}

impl AsRef<str> for PermissionCode {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod permission_code_tests {
    use super::*;
    use platform_kernel::error::{ErrorKind, ErrorMeta};
    use std::str::FromStr;

    #[test]
    fn test_permission_code_valid_cases() {
        let code = PermissionCode::new("  iam:user:add  ").unwrap();
        assert_eq!(code.as_str(), "iam:user:add");
        assert_eq!(code.to_string(), "iam:user:add");

        let code_dash = PermissionCode::new("iam:user-profile:edit").unwrap();
        assert_eq!(code_dash.as_str(), "iam:user-profile:edit");

        let code_128 = "a".repeat(128);
        assert!(PermissionCode::new(&code_128).is_ok());

        let code_from_str = PermissionCode::from_str("iam:role:delete").unwrap();
        assert_eq!(code_from_str.as_str(), "iam:role:delete");
    }

    #[test]
    fn test_permission_code_try_from() {
        let code1: PermissionCode = "iam:user:list".try_into().unwrap();
        assert_eq!(code1.as_str(), "iam:user:list");

        let raw_string = String::from("  iam:menu:view  ");
        let code2: PermissionCode = raw_string.try_into().unwrap();
        assert_eq!(code2.as_str(), "iam:menu:view");

        let err = PermissionCode::try_from("IAM:User:Add").unwrap_err();
        assert_eq!(
            err,
            PermissionCodeError::Invalid {
                value: "IAM:User:Add".to_string()
            }
        );
    }

    #[test]
    fn test_permission_code_invalid_cases() {
        assert_eq!(PermissionCode::new(""), Err(PermissionCodeError::Empty));
        assert_eq!(PermissionCode::new("   "), Err(PermissionCodeError::Empty));

        let code_129 = "a".repeat(129);
        assert_eq!(
            PermissionCode::new(&code_129),
            Err(PermissionCodeError::TooLong)
        );

        // 大写字母不允许
        assert_eq!(
            PermissionCode::new("IAM:USER:ADD"),
            Err(PermissionCodeError::Invalid {
                value: "IAM:USER:ADD".to_string()
            })
        );
        // 空格、特殊符号不允许
        assert_eq!(
            PermissionCode::new("iam user add"),
            Err(PermissionCodeError::Invalid {
                value: "iam user add".to_string()
            })
        );
    }

    #[test]
    fn test_permission_code_error_meta() {
        let err_too_long = PermissionCodeError::TooLong;
        assert_eq!(err_too_long.kind(), ErrorKind::Validation);
        assert_eq!(err_too_long.code(), "iam.permission.code.too_long");
        assert_eq!(err_too_long.fields()[0].field, "code");

        let err_invalid = PermissionCodeError::Invalid {
            value: "Bad Code".to_string(),
        };
        assert_eq!(err_invalid.code(), "iam.permission.code.invalid");
        assert_eq!(
            err_invalid.detail().unwrap(),
            "权限编码格式无效: 'Bad Code'"
        );
    }
}
