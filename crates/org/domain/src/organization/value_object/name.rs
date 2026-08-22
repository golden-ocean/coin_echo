use std::{fmt, str::FromStr};

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OrganizationNameError {
    #[error("组织名称不能为空")]
    Empty,
    #[error("组织名称过长，最大允许 64 个字符")]
    TooLong,
}

impl ErrorMeta for OrganizationNameError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Validation
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Empty => "org.organization.name.empty",
            Self::TooLong => "org.organization.name.too_long",
        }
    }

    fn fields(&self) -> Vec<FieldError> {
        vec![FieldError::new("name", self.code())]
    }
}

/// 组织名称 值对象
///
/// # 业务约束
/// 1. 首尾空白自动 trim
/// 2. 非空
/// 3. 最大长度 64
/// 4. 全局唯一（由团队命名规范约定：子组织命名需带能区分归属的前缀，如"北京财务部"）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizationName(String);

impl OrganizationName {
    pub fn new(s: impl Into<String>) -> Result<Self, OrganizationNameError> {
        let trimmed = s.into().trim().to_string();
        if trimmed.is_empty() {
            return Err(OrganizationNameError::Empty);
        }
        if trimmed.chars().count() > 64 {
            return Err(OrganizationNameError::TooLong);
        }
        Ok(Self(trimmed))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for OrganizationName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for OrganizationName {
    type Err = OrganizationNameError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<&str> for OrganizationName {
    type Error = OrganizationNameError;
    #[inline]
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl TryFrom<String> for OrganizationName {
    type Error = OrganizationNameError;
    #[inline]
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl TryFrom<&String> for OrganizationName {
    type Error = OrganizationNameError;
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
        let name = OrganizationName::new("  北京财务部  ").unwrap();
        assert_eq!(name.as_str(), "北京财务部");
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(OrganizationName::new(""), Err(OrganizationNameError::Empty));
        assert_eq!(
            OrganizationName::new("   "),
            Err(OrganizationNameError::Empty)
        );
    }

    #[test]
    fn rejects_too_long() {
        let too_long = "字".repeat(65);
        assert_eq!(
            OrganizationName::new(&too_long),
            Err(OrganizationNameError::TooLong)
        );
    }

    #[test]
    fn accepts_exactly_64_chars() {
        let exactly_64 = "字".repeat(64);
        assert!(OrganizationName::new(&exactly_64).is_ok());
    }
}
