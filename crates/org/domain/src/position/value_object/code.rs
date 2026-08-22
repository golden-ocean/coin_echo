use std::{fmt, str::FromStr};

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PositionCodeError {
    #[error("职位编码不能为空")]
    Empty,
    #[error("职位编码过长，最大允许 64 个字符")]
    TooLong,
    #[error("职位编码格式无效: {value}，仅允许字母、数字、下划线、中划线")]
    Invalid { value: String },
}

impl ErrorMeta for PositionCodeError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Validation
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Empty => "org.position.code.empty",
            Self::TooLong => "org.position.code.too_long",
            Self::Invalid { .. } => "org.position.code.invalid",
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
pub struct PositionCode(String);

impl PositionCode {
    pub fn new(s: impl Into<String>) -> Result<Self, PositionCodeError> {
        let trimmed = s.into().trim().to_ascii_lowercase();
        if trimmed.is_empty() {
            return Err(PositionCodeError::Empty);
        }
        if trimmed.len() > 64 {
            return Err(PositionCodeError::TooLong);
        }
        if !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(PositionCodeError::Invalid { value: trimmed });
        }
        Ok(Self(trimmed))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PositionCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for PositionCode {
    type Err = PositionCodeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<&str> for PositionCode {
    type Error = PositionCodeError;
    #[inline]
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl TryFrom<String> for PositionCode {
    type Error = PositionCodeError;
    #[inline]
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl TryFrom<&String> for PositionCode {
    type Error = PositionCodeError;
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
        let code = PositionCode::new("Finance").unwrap();
        assert_eq!(code.as_str(), "finance");
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(PositionCode::new(""), Err(PositionCodeError::Empty));
    }

    #[test]
    fn rejects_too_long() {
        let too_long = "a".repeat(65);
        assert_eq!(
            PositionCode::new(&too_long),
            Err(PositionCodeError::TooLong)
        );
    }

    #[test]
    fn rejects_invalid_chars() {
        let err = PositionCode::new("beijing@finance").unwrap_err();
        assert!(matches!(err, PositionCodeError::Invalid { .. }));
    }

    #[test]
    fn accepts_underscore_and_hyphen() {
        assert!(PositionCode::new("beijing_finance-01").is_ok());
    }
}
