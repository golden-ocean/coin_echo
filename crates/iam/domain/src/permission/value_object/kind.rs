use std::{fmt, str::FromStr};

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};

/// 权限类型校验错误枚举
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PermissionKindError {
    #[error("未知的权限类型: {value}")]
    Unknown { value: String },
}

impl ErrorMeta for PermissionKindError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Validation
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Unknown { .. } => "iam.permission.kind_unknown",
        }
    }

    fn detail(&self) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            Self::Unknown { value } => Some(format!("未知的权限类型: '{value}'").into()),
        }
    }

    fn fields(&self) -> Vec<FieldError> {
        vec![FieldError::new("kind", "invalid_enum")]
    }
}

/// 权限类型 值对象 VO：菜单 / 按钮 / 接口
///
/// # 创建方式
/// - PermissionKind::from_str(&str) -> Result<Self, PermissionKindError>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PermissionKind {
    Menu,
    Button,
    Api,
}

impl PermissionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Menu => "menu",
            Self::Button => "button",
            Self::Api => "api",
        }
    }

    pub fn is_menu(&self) -> bool {
        matches!(self, Self::Menu)
    }

    pub fn is_button(&self) -> bool {
        matches!(self, Self::Button)
    }

    pub fn is_api(&self) -> bool {
        matches!(self, Self::Api)
    }
}

impl fmt::Display for PermissionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PermissionKind {
    type Err = PermissionKindError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "menu" => Ok(Self::Menu),
            "button" => Ok(Self::Button),
            "api" => Ok(Self::Api),
            other => Err(PermissionKindError::Unknown {
                value: other.to_string(),
            }),
        }
    }
}

impl TryFrom<&str> for PermissionKind {
    type Error = PermissionKindError;

    #[inline]
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s)
    }
}

impl TryFrom<String> for PermissionKind {
    type Error = PermissionKindError;

    #[inline]
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::from_str(&s)
    }
}

impl AsRef<str> for PermissionKind {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod permission_kind_tests {
    use super::*;
    use platform_kernel::error::{ErrorKind, ErrorMeta};
    use std::str::FromStr;

    #[test]
    fn test_permission_kind_valid_cases() {
        assert_eq!(
            PermissionKind::from_str("menu").unwrap(),
            PermissionKind::Menu
        );
        assert_eq!(
            PermissionKind::from_str("button").unwrap(),
            PermissionKind::Button
        );
        assert_eq!(
            PermissionKind::from_str("api").unwrap(),
            PermissionKind::Api
        );

        assert_eq!(PermissionKind::Menu.as_str(), "menu");
        assert_eq!(PermissionKind::Menu.to_string(), "menu");
        assert!(PermissionKind::Menu.is_menu());
        assert!(PermissionKind::Button.is_button());
        assert!(PermissionKind::Api.is_api());
    }

    #[test]
    fn test_permission_kind_try_from() {
        let kind1: PermissionKind = "api".try_into().unwrap();
        assert_eq!(kind1, PermissionKind::Api);

        let raw_string = String::from("menu");
        let kind2: PermissionKind = raw_string.try_into().unwrap();
        assert_eq!(kind2, PermissionKind::Menu);

        // 大写形式不再合法（统一改为小写后，大写应被拒绝）
        let err = PermissionKind::try_from("Menu").unwrap_err();
        assert_eq!(
            err,
            PermissionKindError::Unknown {
                value: "Menu".to_string()
            }
        );
    }

    #[test]
    fn test_permission_kind_invalid_cases() {
        assert_eq!(
            PermissionKind::from_str(""),
            Err(PermissionKindError::Unknown {
                value: "".to_string()
            })
        );
        assert_eq!(
            PermissionKind::from_str("Page"),
            Err(PermissionKindError::Unknown {
                value: "Page".to_string()
            })
        );
        // 大小写混用同样非法
        assert_eq!(
            PermissionKind::from_str("Api"),
            Err(PermissionKindError::Unknown {
                value: "Api".to_string()
            })
        );
    }

    #[test]
    fn test_permission_kind_error_meta() {
        let err = PermissionKindError::Unknown {
            value: "Widget".to_string(),
        };
        assert_eq!(err.kind(), ErrorKind::Validation);
        assert_eq!(err.code(), "iam.permission.kind_unknown");
        assert_eq!(err.fields()[0].field, "kind");
        assert_eq!(err.detail().unwrap(), "未知的权限类型: 'Widget'");
    }
}
