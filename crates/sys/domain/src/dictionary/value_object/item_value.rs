use std::fmt;
use std::str::FromStr;

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DictionaryItemValueError {
    #[error("字典项键值不能为空")]
    Empty,
    #[error("字典项键值过长：{len}，最大允许 100 个字符")]
    TooLong { len: usize },
    #[error("字典项键值包含非法字符：{value}")]
    Invalid { value: String },
}

impl ErrorMeta for DictionaryItemValueError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Validation
    }
    fn code(&self) -> &'static str {
        match self {
            Self::Empty => "sys.dictionary_item.value.empty",
            Self::TooLong { .. } => "sys.dictionary_item.value.too_long",
            Self::Invalid { .. } => "sys.dictionary_item.value.invalid",
        }
    }
    fn fields(&self) -> Vec<FieldError> {
        vec![FieldError::new("value", self.code())]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictionaryItemValue(String);

impl DictionaryItemValue {
    pub fn new(value: impl Into<String>) -> Result<Self, DictionaryItemValueError> {
        let s = value.into();
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(DictionaryItemValueError::Empty);
        }
        if trimmed.len() > 100 {
            return Err(DictionaryItemValueError::TooLong { len: trimmed.len() });
        }
        // 允许字母、数字、下划线、中划线和点号（通常能满足绝大多数业务编码需求）
        if !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
        {
            return Err(DictionaryItemValueError::Invalid {
                value: trimmed.to_string(),
            });
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DictionaryItemValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for DictionaryItemValue {
    type Err = DictionaryItemValueError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<&str> for DictionaryItemValue {
    type Error = DictionaryItemValueError;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for DictionaryItemValue {
    type Error = DictionaryItemValueError;

    #[inline]
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&String> for DictionaryItemValue {
    type Error = DictionaryItemValueError;

    #[inline]
    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::new(value.as_str())
    }
}
