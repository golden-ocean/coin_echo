use std::fmt;
use std::str::FromStr;

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};

/// DataScope 解析细分错误：空输入 / 格式非法
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DataScopeError {
    #[error("权限范围不能为空")]
    Empty,
    #[error(
        "权限范围格式无效: {value}，合法值：self_only / department / department_and_children / custom / all"
    )]
    Invalid { value: String },
}

impl ErrorMeta for DataScopeError {
    fn kind(&self) -> ErrorKind {
        // 两个变体都是字段格式问题，统一归为调用方输入错误。
        ErrorKind::Validation
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Empty => "iam.user.data_scope.empty",
            Self::Invalid { .. } => "iam.user.data_scope.invalid",
        }
    }

    fn detail(&self) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            Self::Invalid { value } => Some(
                format!(
                    "权限范围格式无效: {value}，合法值：self_only / department / department_and_children / custom / all"
                )
                .into(),
            ),
            Self::Empty => None,
        }
    }

    /// 携带字段名，方便客户端把错误精确定位到表单的哪个输入框。
    fn fields(&self) -> Vec<FieldError> {
        let code = match self {
            Self::Empty => "required",
            Self::Invalid { .. } => "invalid_enum_value",
        };
        vec![FieldError::new("data_scope", code)]
    }
}

/// 数据权限范围枚举
/// 控制接口/数据查询时可见的数据行范围，用于后台RBAC数据权限控制
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Default)]
pub enum DataScope {
    /// 仅查看自己创建的数据（系统默认权限）
    #[default]
    SelfOnly,
    /// 仅本部门数据，不含下级子部门
    Department,
    /// 本部门 + 所有下级子部门数据
    DepartmentAndChildren,
    /// 自定义指定多个部门ID
    Custom,
    /// 全机构所有数据，无数据隔离
    All,
}

impl DataScope {
    /// 获取枚举对应的静态小写存储字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SelfOnly => "self_only",
            Self::Department => "department",
            Self::DepartmentAndChildren => "department_and_children",
            Self::Custom => "custom",
            Self::All => "all",
        }
    }
}

impl fmt::Display for DataScope {
    /// 输出下划线小写字符串，用于存储、前端传输、日志打印
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for DataScope {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for DataScope {
    type Err = DataScopeError;

    /// 从字符串解析权限范围
    /// 自动去除首尾空白、忽略大小写，兼容前端不规则传参
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let raw = s.trim().to_ascii_lowercase();
        if raw.is_empty() {
            return Err(DataScopeError::Empty);
        }
        match raw.as_str() {
            "self_only" => Ok(Self::SelfOnly),
            "department" => Ok(Self::Department),
            "department_and_children" => Ok(Self::DepartmentAndChildren),
            "custom" => Ok(Self::Custom),
            "all" => Ok(Self::All),
            _ => Err(DataScopeError::Invalid { value: raw }),
        }
    }
}

impl TryFrom<&str> for DataScope {
    type Error = DataScopeError;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}

impl TryFrom<String> for DataScope {
    type Error = DataScopeError;

    #[inline]
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl TryFrom<&String> for DataScope {
    type Error = DataScopeError;

    #[inline]
    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::from_str(value.as_str())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    /// 测试Default默认值为SelfOnly
    #[test]
    fn test_default_scope() {
        let scope = DataScope::default();
        assert_eq!(scope, DataScope::SelfOnly);
    }

    /// 测试Display格式化输出字符串完全匹配约定标识
    #[test]
    fn test_display_format() {
        let cases = [
            (DataScope::SelfOnly, "self_only"),
            (DataScope::Department, "department"),
            (DataScope::DepartmentAndChildren, "department_and_children"),
            (DataScope::Custom, "custom"),
            (DataScope::All, "all"),
        ];
        for (scope, expect) in cases {
            assert_eq!(scope.to_string(), expect);
        }
    }

    /// FromStr 正常解析全量合法字符串
    #[test]
    fn test_from_str_valid() {
        let cases = [
            ("self_only", DataScope::SelfOnly),
            ("DEPARTMENT", DataScope::Department),
            (
                "  Department_And_Children  ",
                DataScope::DepartmentAndChildren,
            ),
            ("CUSTOM", DataScope::Custom),
            ("ALL", DataScope::All),
        ];
        for (value, expect) in cases {
            let parsed = DataScope::from_str(value).unwrap();
            assert_eq!(parsed, expect);
        }
    }

    /// 空字符串 → Empty 错误
    #[test]
    fn test_from_str_empty() {
        let err = DataScope::from_str("   ").unwrap_err();
        assert_eq!(err, DataScopeError::Empty);
        assert!(err.to_string().contains("不能为空"));
    }

    /// 非法字符串 → Invalid 错误
    #[test]
    fn test_from_str_invalid() {
        let bad_inputs = ["test", "admin", "dept", "allll"];
        for s in bad_inputs {
            let err = DataScope::from_str(s).unwrap_err();
            assert_eq!(
                err,
                DataScopeError::Invalid {
                    value: s.to_string()
                }
            );
            assert!(err.to_string().contains("权限范围格式无效"));
        }
    }

    /// TryFrom<&str> 和 TryFrom<String> 的测试
    #[test]
    fn test_try_from_str_and_string() {
        // &str
        let scope_ref: DataScope = "department".try_into().unwrap();
        assert_eq!(scope_ref, DataScope::Department);

        // String
        let scope_string: DataScope = String::from("ALL").try_into().unwrap();
        assert_eq!(scope_string, DataScope::All);

        // &String
        let s = String::from("custom");
        let scope_ref_string: DataScope = (&s).try_into().unwrap();
        assert_eq!(scope_ref_string, DataScope::Custom);

        // Error path via TryFrom
        let err: Result<DataScope, _> = "invalid_scope".try_into();
        assert_eq!(
            err.unwrap_err(),
            DataScopeError::Invalid {
                value: "invalid_scope".to_string()
            }
        );
    }

    /// 枚举相等、哈希、拷贝特性校验
    #[test]
    fn test_copy_eq_hash() {
        let s1 = DataScope::All;
        let s2 = s1; // Copy
        assert_eq!(s1, s2);

        let mut map = HashSet::new();
        map.insert(s1);
        assert!(map.contains(&s2));
    }

    #[test]
    fn error_meta_kind_is_always_validation() {
        assert_eq!(DataScopeError::Empty.kind(), ErrorKind::Validation);
        assert_eq!(
            DataScopeError::Invalid { value: "x".into() }.kind(),
            ErrorKind::Validation
        );
    }

    #[test]
    fn error_meta_codes_are_distinct() {
        let empty = DataScopeError::Empty.code();
        let invalid = DataScopeError::Invalid { value: "x".into() }.code();
        assert_ne!(empty, invalid);
        assert!(empty.starts_with("iam.user.data_scope."));
    }

    #[test]
    fn error_meta_fields_names_the_data_scope_field() {
        let fields = DataScopeError::Invalid { value: "x".into() }.fields();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].field, "data_scope");
        assert_eq!(fields[0].code, "invalid_enum_value");
    }
}
