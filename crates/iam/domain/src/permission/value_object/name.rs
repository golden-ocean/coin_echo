use std::{fmt, str::FromStr};

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};

/// 权限名称校验错误枚举
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PermissionNameError {
    #[error("权限名称不能为空")]
    Empty,
    #[error("权限名称过长")]
    TooLong,
    #[error("权限名称格式无效: {value}")]
    Invalid { value: String },
}

impl ErrorMeta for PermissionNameError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Validation
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Empty => "iam.permission.name_empty",
            Self::TooLong => "iam.permission.name_too_long",
            Self::Invalid { .. } => "iam.permission.name_invalid",
        }
    }

    fn detail(&self) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            Self::Invalid { value } => Some(format!("权限名称格式无效: '{value}'").into()),
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

/// 权限名称 值对象 VO
///
/// # 业务约束
/// 1. 首尾空白自动trim
/// 2. 非空
/// 3. 最大长度64个字符
/// 4. 仅允许中文、字母、数字、下划线
///
/// # 创建方式
/// - PermissionName::new(&str) -> Result<Self, PermissionNameError>
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionName(String);

impl PermissionName {
    const MAX_LEN: usize = 64;

    pub fn new(s: impl Into<String>) -> Result<Self, PermissionNameError> {
        let raw = s.into();
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(PermissionNameError::Empty);
        }
        if trimmed.chars().count() > Self::MAX_LEN {
            return Err(PermissionNameError::TooLong);
        }
        // 允许中文、字母、数字、下划线
        if !trimmed.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(PermissionNameError::Invalid {
                value: trimmed.to_string(),
            });
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PermissionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PermissionName {
    type Err = PermissionNameError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<&str> for PermissionName {
    type Error = PermissionNameError;

    #[inline]
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl TryFrom<String> for PermissionName {
    type Error = PermissionNameError;

    #[inline]
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(&s)
    }
}

impl TryFrom<&String> for PermissionName {
    type Error = PermissionNameError;

    #[inline]
    fn try_from(s: &String) -> Result<Self, Self::Error> {
        Self::new(s.as_str())
    }
}

impl AsRef<str> for PermissionName {
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
    fn test_permission_name_valid_cases() {
        // 首尾空格 trim，保留原中文/大小写
        let name = PermissionName::new("  用户管理_01  ").unwrap();
        assert_eq!(name.as_str(), "用户管理_01");
        assert_eq!(name.to_string(), "用户管理_01");

        // 包含英文、数字、下划线
        let name_en = PermissionName::new("User_Manage_01").unwrap();
        assert_eq!(name_en.as_str(), "User_Manage_01");

        // UTF-8 字符计长：64 个汉字合法
        let name_64_cn = "一".repeat(64);
        assert!(PermissionName::new(&name_64_cn).is_ok());

        // FromStr 特征校验
        let name_from_str = PermissionName::from_str("新增用户").unwrap();
        assert_eq!(name_from_str.as_str(), "新增用户");
    }

    #[test]
    fn test_permission_name_try_from() {
        // 1. &str 类型的 try_into
        let name1: PermissionName = "用户管理".try_into().unwrap();
        assert_eq!(name1.as_str(), "用户管理");

        // 2. String 类型的 try_into
        let raw_string = String::from("  角色管理  ");
        let name2: PermissionName = raw_string.try_into().unwrap();
        assert_eq!(name2.as_str(), "角色管理");

        // 3. TryFrom 显式调用失败用例
        let err = PermissionName::try_from("用户-管理").unwrap_err();
        assert_eq!(
            err,
            PermissionNameError::Invalid {
                value: "用户-管理".to_string()
            }
        );
    }

    #[test]
    fn test_permission_name_invalid_cases() {
        // 1. 空或纯空格
        assert_eq!(PermissionName::new(""), Err(PermissionNameError::Empty));
        assert_eq!(PermissionName::new("   "), Err(PermissionNameError::Empty));

        // 2. 超过 64 个 UTF-8 字符（65 个汉字）
        let name_65_cn = "一".repeat(65);
        assert_eq!(
            PermissionName::new(&name_65_cn),
            Err(PermissionNameError::TooLong)
        );

        // 3. 非法字符（包含连字符、特殊符号、空格等）
        assert_eq!(
            PermissionName::new("用户-管理"),
            Err(PermissionNameError::Invalid {
                value: "用户-管理".to_string()
            })
        );
        assert_eq!(
            PermissionName::new("Perm #1"),
            Err(PermissionNameError::Invalid {
                value: "Perm #1".to_string()
            })
        );
    }

    #[test]
    fn test_permission_name_error_meta() {
        let err_too_long = PermissionNameError::TooLong;
        assert_eq!(err_too_long.kind(), ErrorKind::Validation);
        assert_eq!(err_too_long.code(), "iam.permission.name_too_long");
        assert_eq!(err_too_long.fields()[0].field, "name");

        let err_invalid = PermissionNameError::Invalid {
            value: "name#".to_string(),
        };
        assert_eq!(err_invalid.code(), "iam.permission.name_invalid");
        assert_eq!(err_invalid.detail().unwrap(), "权限名称格式无效: 'name#'");
    }
}
