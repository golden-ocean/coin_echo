use std::{fmt, str::FromStr};

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OrganizationCodeError {
    #[error("组织编码不能为空")]
    Empty,
    #[error("组织编码过长，最大允许 64 个字符")]
    TooLong,
    #[error("组织编码格式无效: {value}，仅允许字母、数字、下划线、中划线")]
    Invalid { value: String },
}

impl ErrorMeta for OrganizationCodeError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Validation
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Empty => "org.organization.code.empty",
            Self::TooLong => "org.organization.code.too_long",
            Self::Invalid { .. } => "org.organization.code.invalid",
        }
    }

    fn fields(&self) -> Vec<FieldError> {
        vec![FieldError::new("code", self.code())]
    }
}

/// 组织编码 值对象
///
/// # 业务约束
/// 1. 首尾空白自动 trim，统一转小写存储
/// 2. 非空，最大长度 64
/// 3. 仅允许字母、数字、下划线、中划线（如 beijing-finance）
/// 4. 全局唯一，命名规范同 OrganizationName
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizationCode(String);

impl OrganizationCode {
    pub fn new(s: impl Into<String>) -> Result<Self, OrganizationCodeError> {
        let trimmed = s.into().trim().to_ascii_lowercase();
        if trimmed.is_empty() {
            return Err(OrganizationCodeError::Empty);
        }
        if trimmed.len() > 64 {
            return Err(OrganizationCodeError::TooLong);
        }
        if !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(OrganizationCodeError::Invalid { value: trimmed });
        }
        Ok(Self(trimmed))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for OrganizationCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for OrganizationCode {
    type Err = OrganizationCodeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<&str> for OrganizationCode {
    type Error = OrganizationCodeError;
    #[inline]
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl TryFrom<String> for OrganizationCode {
    type Error = OrganizationCodeError;
    #[inline]
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl TryFrom<&String> for OrganizationCode {
    type Error = OrganizationCodeError;
    #[inline]
    fn try_from(s: &String) -> Result<Self, Self::Error> {
        Self::new(s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_to_lowercase() {
        let code = OrganizationCode::new("Beijing-Finance").unwrap();
        assert_eq!(code.as_str(), "beijing-finance");
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(OrganizationCode::new(""), Err(OrganizationCodeError::Empty));
    }

    #[test]
    fn rejects_too_long() {
        let too_long = "a".repeat(65);
        assert_eq!(
            OrganizationCode::new(&too_long),
            Err(OrganizationCodeError::TooLong)
        );
    }

    #[test]
    fn rejects_invalid_chars() {
        let err = OrganizationCode::new("beijing@finance").unwrap_err();
        assert!(matches!(err, OrganizationCodeError::Invalid { .. }));
    }

    #[test]
    fn accepts_underscore_and_hyphen() {
        assert!(OrganizationCode::new("beijing_finance-01").is_ok());
    }
}
