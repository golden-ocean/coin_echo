use std::{fmt, str::FromStr};

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PositionNameError {
    #[error("职位名称不能为空")]
    Empty,
    #[error("职位名称过长，最大允许 64 个字符")]
    TooLong,
}

impl ErrorMeta for PositionNameError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Validation
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Empty => "org.position.name.empty",
            Self::TooLong => "org.position.name.too_long",
        }
    }

    fn fields(&self) -> Vec<FieldError> {
        vec![FieldError::new("name", self.code())]
    }
}

/// 职位名称 值对象
///
/// # 业务约束
/// 1. 首尾空白自动 trim
/// 2. 非空
/// 3. 最大长度 64
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionName(String);

impl PositionName {
    pub fn new(s: impl Into<String>) -> Result<Self, PositionNameError> {
        let trimmed = s.into().trim().to_string();
        if trimmed.is_empty() {
            return Err(PositionNameError::Empty);
        }
        if trimmed.chars().count() > 64 {
            return Err(PositionNameError::TooLong);
        }
        Ok(Self(trimmed))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PositionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for PositionName {
    type Err = PositionNameError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<&str> for PositionName {
    type Error = PositionNameError;
    #[inline]
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl TryFrom<String> for PositionName {
    type Error = PositionNameError;
    #[inline]
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl TryFrom<&String> for PositionName {
    type Error = PositionNameError;
    #[inline]
    fn try_from(s: &String) -> Result<Self, Self::Error> {
        Self::new(s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_and_accepts_valid_name() {
        let name = PositionName::new("  财务主管  ").unwrap();
        assert_eq!(name.as_str(), "财务主管");
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(PositionName::new(""), Err(PositionNameError::Empty));
        assert_eq!(PositionName::new("   "), Err(PositionNameError::Empty));
    }

    #[test]
    fn rejects_too_long() {
        let too_long = "字".repeat(65);
        assert_eq!(
            PositionName::new(&too_long),
            Err(PositionNameError::TooLong)
        );
    }

    #[test]
    fn accepts_exactly_64_chars() {
        let exactly_64 = "字".repeat(64);
        assert!(PositionName::new(&exactly_64).is_ok());
    }
}
