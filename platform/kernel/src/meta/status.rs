use std::fmt;
use std::str::FromStr;

use crate::error::{ErrorKind, ErrorMeta, FieldError};

/// 状态字符串解析错误
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StatusError {
    #[error("状态标识不能为空")]
    Empty,
    #[error("状态标识无效: {value}, 合法值：enabled / disabled")]
    Invalid { value: String },
}

impl ErrorMeta for StatusError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Validation
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Empty => "status.empty",
            Self::Invalid { .. } => "status.invalid",
        }
    }

    fn detail(&self) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            Self::Invalid { value } => {
                Some(format!("状态标识无效: {value}，合法值：enabled / disabled").into())
            }
            Self::Empty => None,
        }
    }

    fn fields(&self) -> Vec<FieldError> {
        let code = match self {
            Self::Empty => "required",
            Self::Invalid { .. } => "invalid_enum_value",
        };
        vec![FieldError::new("status", code)]
    }
}

/// 通用启用/禁用状态枚举
/// 适用于用户、角色、部门、接口开关等各类功能启停标记
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Status {
    /// 禁用，功能/数据关闭不可用
    Disabled,
    /// 启用，正常生效（系统默认状态）
    #[default]
    Enabled,
}

impl Status {
    /// 是否为启用状态
    pub fn is_enabled(&self) -> bool {
        matches!(self, Self::Enabled)
    }

    /// 是否为禁用状态
    pub fn is_disabled(&self) -> bool {
        matches!(self, Self::Disabled)
    }

    /// 获取枚举对应的静态小写存储字符串，用于数据库、接口序列化
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
        }
    }

    /// 私有核心解析逻辑，统一复用，消除重复代码
    fn parse_raw(raw_input: &str) -> Result<Self, StatusError> {
        let raw = raw_input.trim().to_ascii_lowercase();
        if raw.is_empty() {
            return Err(StatusError::Empty);
        }
        match raw.as_str() {
            "enabled" => Ok(Self::Enabled),
            "disabled" => Ok(Self::Disabled),
            _ => Err(StatusError::Invalid { value: raw }),
        }
    }
}

/// Display：输出标准存储字符串，日志、打印直接使用
impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// FromStr 标准解析 Trait，支持 `"enabled".parse::<Status>()`
impl FromStr for Status {
    type Err = StatusError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_raw(s)
    }
}

/// AsRef<str> 通用引用转换，兼容接收 &str 的泛型工具函数
impl AsRef<str> for Status {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl TryFrom<&str> for Status {
    type Error = StatusError;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse_raw(value)
    }
}

impl TryFrom<String> for Status {
    type Error = StatusError;

    #[inline]
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse_raw(&value)
    }
}

impl TryFrom<&String> for Status {
    type Error = StatusError;

    #[inline]
    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::parse_raw(value.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== 基础默认值、布尔判断测试 ==========
    #[test]
    fn test_default_value() {
        let status = Status::default();
        assert_eq!(status, Status::Enabled);
        assert!(status.is_enabled());
        assert!(!status.is_disabled());
    }

    #[test]
    fn test_bool_judge_method() {
        let enable = Status::Enabled;
        assert!(enable.is_enabled());
        assert!(!enable.is_disabled());

        let disable = Status::Disabled;
        assert!(disable.is_disabled());
        assert!(!disable.is_enabled());
    }

    // ========== 字符串解析系列测试 ==========
    #[test]
    fn test_parse_valid_input_ignore_case_blank() {
        // 兼容大小写、首尾空格
        let test_cases = [
            ("enabled", Status::Enabled),
            ("ENABLED", Status::Enabled),
            ("  Enabled  ", Status::Enabled),
            ("disabled", Status::Disabled),
            ("DISABLED", Status::Disabled),
            (" Disabled ", Status::Disabled),
        ];

        for (input, expect) in test_cases {
            // FromStr::from_str 借用 &str
            assert_eq!(Status::from_str(input), Ok(expect));
            // parse 语法糖（内部走 from_str）
            assert_eq!(input.parse::<Status>(), Ok(expect));
        }
    }

    #[test]
    fn test_parse_empty_input_error() {
        assert_eq!(Status::from_str(""), Err(StatusError::Empty));
        assert_eq!(Status::from_str("   "), Err(StatusError::Empty));
    }

    #[test]
    fn test_parse_invalid_input_error() {
        let err = Status::from_str("active").unwrap_err();
        assert_eq!(
            err,
            StatusError::Invalid {
                value: "active".into()
            }
        );

        // 大小写归一后仍无法识别
        let err = Status::from_str("ENABLE").unwrap_err();
        assert!(matches!(err, StatusError::Invalid { value } if value == "enable"));
    }

    // ========== TryFrom 解析系列测试 ==========
    #[test]
    fn test_try_from_str_and_string() {
        // &str 方式转换
        let status_ref: Status = "enabled".try_into().unwrap();
        assert_eq!(status_ref, Status::Enabled);

        let status_ref_upper: Status = "DISABLED".try_into().unwrap();
        assert_eq!(status_ref_upper, Status::Disabled);

        // String 方式转换
        let status_string: Status = String::from("enabled").try_into().unwrap();
        assert_eq!(status_string, Status::Enabled);

        // &String 方式转换
        let s = String::from("disabled");
        let status_ref_string: Status = (&s).try_into().unwrap();
        assert_eq!(status_ref_string, Status::Disabled);

        // TryFrom 错误路径测试
        let err: Result<Status, _> = "unknown".try_into();
        assert_eq!(
            err.unwrap_err(),
            StatusError::Invalid {
                value: "unknown".to_string()
            }
        );
    }

    // ========== 字符串转换一致性测试 ==========
    #[test]
    fn test_string_convert_consistent() {
        let cases = [(Status::Enabled, "enabled"), (Status::Disabled, "disabled")];
        for (status, expect_str) in cases {
            assert_eq!(status.as_str(), expect_str);
            assert_eq!(status.to_string(), expect_str);
            assert_eq!(status.as_ref(), expect_str);
            let s: String = status.to_string();
            assert_eq!(s, expect_str);
        }
    }

    // ========== 错误 Display 测试 ==========
    #[test]
    fn test_status_error_display() {
        assert_eq!(StatusError::Empty.to_string(), "状态标识不能为空");

        let invalid = StatusError::Invalid { value: "on".into() };
        let msg = invalid.to_string();
        assert!(msg.contains("on"));
        assert!(msg.contains("enabled / disabled"));
    }

    #[test]
    fn test_status_error_errormeta() {
        let empty = StatusError::Empty;
        assert_eq!(empty.kind(), ErrorKind::Validation);
        assert_eq!(empty.code(), "status.empty");
        assert!(empty.detail().is_none());
        assert!(!empty.retryable());

        let invalid = StatusError::Invalid { value: "on".into() };
        assert_eq!(invalid.kind(), ErrorKind::Validation);
        assert_eq!(invalid.code(), "status.invalid");
        let detail = invalid.detail();
        assert!(detail.as_deref().unwrap().contains("on"));
        assert!(!invalid.retryable());
    }

    #[test]
    fn test_status_error_fields() {
        // Empty → 字段必填
        let empty_fields = StatusError::Empty.fields();
        assert_eq!(empty_fields.len(), 1);
        assert_eq!(empty_fields[0].field.as_ref(), "status");
        assert_eq!(empty_fields[0].code, "required");

        // Invalid → 非法枚举值
        let invalid = StatusError::Invalid { value: "on".into() };
        let invalid_fields = invalid.fields();
        assert_eq!(invalid_fields.len(), 1);
        assert_eq!(invalid_fields[0].field.as_ref(), "status");
        assert_eq!(invalid_fields[0].code, "invalid_enum_value");
    }
}
