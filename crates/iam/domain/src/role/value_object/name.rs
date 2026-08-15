use std::{fmt, str::FromStr};

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};

/// 角色名称校验错误枚举
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RoleNameError {
    #[error("角色名称不能为空")]
    Empty,
    #[error("角色名称过长")]
    TooLong,
    #[error("角色名称格式无效: {value}")]
    Invalid { value: String },
}

impl ErrorMeta for RoleNameError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Validation
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Empty => "iam.role.name.empty",
            Self::TooLong => "iam.role.name.too_long",
            Self::Invalid { .. } => "iam.role.name.invalid",
        }
    }

    fn detail(&self) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            Self::Invalid { value } => Some(format!("角色名称格式无效: '{value}'").into()),
            _ => None,
        }
    }

    fn fields(&self) -> Vec<FieldError> {
        let code = match self {
            Self::Empty => "required",
            Self::TooLong => "too_long",
            Self::Invalid { .. } => "invalid_format",
        };
        vec![FieldError::new("name", code)]
    }
}

/// 角色名称 值对象 VO
///
/// # 业务约束
/// 1. 首尾空白自动trim
/// 2. 非空
/// 3. 最大长度64个字符
/// 4. 仅允许中文、字母、数字、下划线
///
/// # 创建方式
/// - RoleName::new(&str) -> Result<Self, RoleNameError>
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleName(String);

impl RoleName {
    const MAX_LEN: usize = 64;

    pub fn new(s: impl Into<String>) -> Result<Self, RoleNameError> {
        let raw = s.into();
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(RoleNameError::Empty);
        }
        if trimmed.chars().count() > Self::MAX_LEN {
            return Err(RoleNameError::TooLong);
        }
        // 允许中文、字母、数字、下划线
        if !trimmed.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(RoleNameError::Invalid {
                value: trimmed.to_string(),
            });
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RoleName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RoleName {
    type Err = RoleNameError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<&str> for RoleName {
    type Error = RoleNameError;

    #[inline]
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl TryFrom<String> for RoleName {
    type Error = RoleNameError;

    #[inline]
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(&s)
    }
}

impl AsRef<str> for RoleName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_kernel::error::{ErrorKind, ErrorMeta};
    use std::str::FromStr;

    #[test]
    fn test_role_name_valid_cases() {
        // 首尾空格 trim，保留原中文/大小写
        let name = RoleName::new("  系统管理员_01  ").unwrap();
        assert_eq!(name.as_str(), "系统管理员_01");
        assert_eq!(name.to_string(), "系统管理员_01");

        // 包含英文、数字、下划线
        let name_en = RoleName::new("System_Admin_01").unwrap();
        assert_eq!(name_en.as_str(), "System_Admin_01");

        // UTF-8 字符计长：64 个汉字合法
        let name_64_cn = "一".repeat(64);
        assert!(RoleName::new(&name_64_cn).is_ok());

        // FromStr 特征校验
        let name_from_str = RoleName::from_str("运营人员").unwrap();
        assert_eq!(name_from_str.as_str(), "运营人员");
    }

    #[test]
    fn test_role_name_try_from() {
        // 1. &str 类型的 try_into
        let name1: RoleName = "系统管理员".try_into().unwrap();
        assert_eq!(name1.as_str(), "系统管理员");

        // 2. String 类型的 try_into
        let raw_string = String::from("  运营专员  ");
        let name2: RoleName = raw_string.try_into().unwrap();
        assert_eq!(name2.as_str(), "运营专员");

        // 3. TryFrom 显式调用失败用例
        let err = RoleName::try_from("系统-管理员").unwrap_err();
        assert_eq!(
            err,
            RoleNameError::Invalid {
                value: "系统-管理员".to_string()
            }
        );
    }

    #[test]
    fn test_role_name_invalid_cases() {
        // 1. 空或纯空格
        assert_eq!(RoleName::new(""), Err(RoleNameError::Empty));
        assert_eq!(RoleName::new("   "), Err(RoleNameError::Empty));

        // 2. 超过 64 个 UTF-8 字符（65 个汉字）
        let name_65_cn = "一".repeat(65);
        assert_eq!(RoleName::new(&name_65_cn), Err(RoleNameError::TooLong));

        // 3. 非法字符（包含连字符、特殊符号、空格等）
        assert_eq!(
            RoleName::new("系统-管理员"),
            Err(RoleNameError::Invalid {
                value: "系统-管理员".to_string()
            })
        );
        assert_eq!(
            RoleName::new("Role #1"),
            Err(RoleNameError::Invalid {
                value: "Role #1".to_string()
            })
        );
    }

    #[test]
    fn test_role_name_error_meta() {
        let err_too_long = RoleNameError::TooLong;
        assert_eq!(err_too_long.kind(), ErrorKind::Validation);
        assert_eq!(err_too_long.code(), "iam.role.name.too_long");
        assert_eq!(err_too_long.fields()[0].field, "name");

        let err_invalid = RoleNameError::Invalid {
            value: "name#".to_string(),
        };
        assert_eq!(err_invalid.code(), "iam.role.name.invalid");
        assert_eq!(err_invalid.detail().unwrap(), "角色名称格式无效: 'name#'");
    }
}
