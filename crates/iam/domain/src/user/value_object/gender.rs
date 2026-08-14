use std::fmt;
use std::str::FromStr;

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};

/// 性别解析错误
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GenderError {
    #[error("性别不能为空")]
    Empty,
    #[error("性别格式无效: {value}, 合法值：male / female / unknown")]
    Invalid { value: String },
}

impl ErrorMeta for GenderError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Validation
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Empty => "iam.user.gender.empty",
            Self::Invalid { .. } => "iam.user.gender.invalid",
        }
    }

    fn detail(&self) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            Self::Invalid { value } => {
                Some(format!("性别格式无效: {value}, 合法值：male / female / unknown").into())
            }
            Self::Empty => None,
        }
    }

    fn fields(&self) -> Vec<FieldError> {
        let code = match self {
            Self::Empty => "required",
            Self::Invalid { .. } => "invalid_enum_value",
        };
        vec![FieldError::new("gender", code)]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Gender {
    #[default]
    Unknown,
    Male,
    Female,
}

impl Gender {
    /// 获取枚举对应的静态小写存储字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Male => "male",
            Self::Female => "female",
        }
    }

    /// 判断是否为男性
    pub fn is_male(&self) -> bool {
        matches!(self, Self::Male)
    }

    /// 判断是否为女性
    pub fn is_female(&self) -> bool {
        matches!(self, Self::Female)
    }

    /// 判断性别是否未知
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }
}

/// Display 实现：输出统一小写字符串，日志、打印、拼接直接使用
impl fmt::Display for Gender {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// FromStr 标准字符串解析，支持 "male".parse::<Gender>()
impl FromStr for Gender {
    type Err = GenderError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let raw = s.trim().to_ascii_lowercase();
        if raw.is_empty() {
            return Err(GenderError::Empty);
        }
        match raw.as_str() {
            "unknown" => Ok(Self::Unknown),
            "male" => Ok(Self::Male),
            "female" => Ok(Self::Female),
            _ => Err(GenderError::Invalid { value: raw }),
        }
    }
}

/// AsRef<str> 通用引用转换，兼容所有接收 &str 的泛型工具函数
impl AsRef<str> for Gender {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl TryFrom<&str> for Gender {
    type Error = GenderError;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}

impl TryFrom<String> for Gender {
    type Error = GenderError;

    #[inline]
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl TryFrom<&String> for Gender {
    type Error = GenderError;

    #[inline]
    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::from_str(value.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 默认值校验，默认未知
    #[test]
    fn test_gender_default() {
        let g = Gender::default();
        assert_eq!(g, Gender::Unknown);
        assert!(g.is_unknown());
    }

    /// FromStr 合法输入，兼容大小写、首尾空格
    #[test]
    fn test_from_str_valid() {
        let cases = [
            ("unknown", Gender::Unknown),
            ("MALE", Gender::Male),
            ("  Female  ", Gender::Female),
        ];
        for (value, expect) in cases {
            let parse1 = Gender::from_str(value).unwrap();
            let parse2 = value.parse::<Gender>().unwrap();
            assert_eq!(parse1, expect);
            assert_eq!(parse2, expect);
        }
    }

    /// 空字符串、全空白返回 Empty 错误
    #[test]
    fn test_from_str_empty_err() {
        let err1 = Gender::from_str("").unwrap_err();
        assert_eq!(err1, GenderError::Empty);

        let err2 = Gender::from_str("    ").unwrap_err();
        assert_eq!(err2, GenderError::Empty);
    }

    /// 非法字符返回 Invalid，错误提示包含合法值
    #[test]
    fn test_from_str_invalid_err() {
        let bad = ["man", "woman", "xxx", "123"];
        for s in bad {
            let err = Gender::from_str(s).unwrap_err();
            assert_eq!(
                err,
                GenderError::Invalid {
                    value: s.to_string()
                }
            );
            assert!(err.to_string().contains(s));
            assert!(err.to_string().contains("性别格式无效"));
        }
    }

    /// TryFrom<&str>、TryFrom<String> 和 TryFrom<&String> 的转换测试
    #[test]
    fn test_try_from_str_and_string() {
        // &str
        let g_ref: Gender = "male".try_into().unwrap();
        assert_eq!(g_ref, Gender::Male);

        // String
        let g_string: Gender = String::from("FEMALE").try_into().unwrap();
        assert_eq!(g_string, Gender::Female);

        // &String
        let s = String::from("unknown");
        let g_ref_string: Gender = (&s).try_into().unwrap();
        assert_eq!(g_ref_string, Gender::Unknown);

        // Error path via TryFrom
        let err: Result<Gender, _> = "other".try_into();
        assert_eq!(
            err.unwrap_err(),
            GenderError::Invalid {
                value: "other".to_string()
            }
        );
    }

    /// as_str / Display / AsRef 输出完全一致
    #[test]
    fn test_string_output_uniform() {
        let list = [
            (Gender::Unknown, "unknown"),
            (Gender::Male, "male"),
            (Gender::Female, "female"),
        ];
        for (g, expect) in list {
            assert_eq!(g.as_str(), expect);
            assert_eq!(g.to_string(), expect);
            assert_eq!(g.as_ref(), expect);
        }
    }

    /// 布尔判断方法校验
    #[test]
    fn test_bool_judge() {
        let male = Gender::Male;
        assert!(male.is_male());
        assert!(!male.is_female());
        assert!(!male.is_unknown());

        let female = Gender::Female;
        assert!(female.is_female());
        assert!(!female.is_male());

        let unknown = Gender::Unknown;
        assert!(unknown.is_unknown());
    }

    // ---- ErrorMeta ----

    #[test]
    fn error_meta_kind_is_always_validation() {
        assert_eq!(GenderError::Empty.kind(), ErrorKind::Validation);
        assert_eq!(
            GenderError::Invalid { value: "x".into() }.kind(),
            ErrorKind::Validation
        );
    }

    #[test]
    fn error_meta_codes_are_distinct_and_namespaced() {
        let empty = GenderError::Empty.code();
        let invalid = GenderError::Invalid { value: "x".into() }.code();
        assert_ne!(empty, invalid);
        assert!(empty.starts_with("iam.user.gender."));
    }

    #[test]
    fn error_meta_fields_names_gender_field() {
        let fields = GenderError::Empty.fields();
        assert_eq!(fields[0].field, "gender");
        assert_eq!(fields[0].code, "required");
    }
}
