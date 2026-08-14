use std::{fmt, str::FromStr};

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};

/// 角色编码校验错误枚举
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RoleCodeError {
    #[error("角色编码不能为空")]
    Empty,
    #[error("角色编码过长")]
    TooLong,
    #[error("角色编码格式无效: {value}")]
    Invalid { value: String },
}

impl ErrorMeta for RoleCodeError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Validation
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Empty => "iam.role.code_empty",
            Self::TooLong => "iam.role.code_too_long",
            Self::Invalid { .. } => "iam.role.code_invalid",
        }
    }

    fn detail(&self) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            Self::Invalid { value } => Some(format!("角色编码格式无效: '{value}'").into()),
            _ => None,
        }
    }

    fn fields(&self) -> Vec<FieldError> {
        let code = match self {
            Self::Empty => "required",
            Self::Invalid { .. } => "invalid_format",
            Self::TooLong => "too_long",
        };
        vec![FieldError::new("code", code)]
    }
}

/// 角色编码 值对象 VO
///
/// # 业务约束
/// 1. 首尾空白自动trim
/// 2. 统一转为小写存储
/// 3. 非空
/// 4. 最大长度64
/// 5. 仅允许字母、数字、下划线 `[a-z0-9_]`
///
/// # 创建方式
/// - RoleCode::new(&str) -> Result<Self, RoleCodeError>
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleCode(String);

impl RoleCode {
    pub fn new(s: &str) -> Result<Self, RoleCodeError> {
        let raw = s.trim().to_ascii_lowercase();
        if raw.is_empty() {
            return Err(RoleCodeError::Empty);
        }
        if raw.len() > 64 {
            return Err(RoleCodeError::TooLong);
        }
        // 允许字母、数字、下划线, 不允许中文
        if !raw.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(RoleCodeError::Invalid { value: raw });
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RoleCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RoleCode {
    type Err = RoleCodeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl AsRef<str> for RoleCode {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl TryFrom<&str> for RoleCode {
    type Error = RoleCodeError;

    #[inline]
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl TryFrom<String> for RoleCode {
    type Error = RoleCodeError;

    #[inline]
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(&s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_kernel::error::{ErrorKind, ErrorMeta};
    use std::str::FromStr;

    #[test]
    fn test_role_code_valid_cases() {
        // 自动 trim 和小写转换
        let code = RoleCode::new("  ADMIN_role_01  ").unwrap();
        assert_eq!(code.as_str(), "admin_role_01");
        assert_eq!(code.to_string(), "admin_role_01");

        // 刚好 64 字符
        let len_64 = "a".repeat(64);
        assert!(RoleCode::new(&len_64).is_ok());

        // FromStr 特征校验
        let code_from_str = RoleCode::from_str("SUPER_ADMIN").unwrap();
        assert_eq!(code_from_str.as_str(), "super_admin");
    }

    #[test]
    fn test_role_code_invalid_cases() {
        // 1. 空或纯空格
        assert_eq!(RoleCode::new(""), Err(RoleCodeError::Empty));
        assert_eq!(RoleCode::new("   "), Err(RoleCodeError::Empty));

        // 2. 超过 64 字符
        let len_65 = "a".repeat(65);
        assert_eq!(RoleCode::new(&len_65), Err(RoleCodeError::TooLong));

        // 3. 非法字符（包含中文、特殊符号、空格等）
        assert_eq!(
            RoleCode::new("admin-role"), // 连字符非法
            Err(RoleCodeError::Invalid {
                value: "admin-role".to_string()
            })
        );
        assert_eq!(
            RoleCode::new("管理员"), // 中文非法
            Err(RoleCodeError::Invalid {
                value: "管理员".to_string()
            })
        );
        assert_eq!(
            RoleCode::new("role@123"), // 特殊符号非法
            Err(RoleCodeError::Invalid {
                value: "role@123".to_string()
            })
        );
    }

    #[test]
    fn test_role_code_error_meta() {
        let err_empty = RoleCodeError::Empty;
        assert_eq!(err_empty.kind(), ErrorKind::Validation);
        assert_eq!(err_empty.code(), "iam.role.code_empty");
        assert_eq!(err_empty.detail(), None);
        assert_eq!(err_empty.fields()[0].field, "code");

        let err_invalid = RoleCodeError::Invalid {
            value: "invalid-code".to_string(),
        };
        assert_eq!(err_invalid.code(), "iam.role.code_invalid");
        assert_eq!(
            err_invalid.detail().unwrap(),
            "角色编码格式无效: 'invalid-code'"
        );
    }

    #[test]
    fn test_role_code_try_from() {
        // 1. &str 类型的 try_into
        let code1: RoleCode = "ADMIN_01".try_into().unwrap();
        assert_eq!(code1.as_str(), "admin_01");

        // 2. String 类型的 try_into
        let raw_string = String::from("  SUPER_USER  ");
        let code2: RoleCode = raw_string.try_into().unwrap();
        assert_eq!(code2.as_str(), "super_user");

        // 3. TryFrom 显式调用失败用例
        let err = RoleCode::try_from("bad-code").unwrap_err();
        assert_eq!(
            err,
            RoleCodeError::Invalid {
                value: "bad-code".to_string()
            }
        );
    }
}
