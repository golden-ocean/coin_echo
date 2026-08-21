use std::fmt;
use std::str::FromStr;

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DictionaryCodeError {
    #[error("字典编码不能为空")]
    Empty,
    #[error("字典编码过长：{len}，最大允许 64 个字符")]
    TooLong { len: usize },
    #[error("字典编码包含非法字符：{value}，仅允许小写字母、数字、下划线")]
    Invalid { value: String },
}

impl ErrorMeta for DictionaryCodeError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Validation
    }
    fn code(&self) -> &'static str {
        match self {
            Self::Empty => "sys.dictionary.code.empty",
            Self::TooLong { .. } => "sys.dictionary.code.too_long",
            Self::Invalid { .. } => "sys.dictionary.code.invalid",
        }
    }
    fn fields(&self) -> Vec<FieldError> {
        vec![FieldError::new("code", self.code())]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictionaryCode(String);

impl DictionaryCode {
    pub fn new(value: impl Into<String>) -> Result<Self, DictionaryCodeError> {
        let s = value.into();
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(DictionaryCodeError::Empty);
        }
        if trimmed.len() > 64 {
            return Err(DictionaryCodeError::TooLong { len: trimmed.len() });
        }
        if !trimmed
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            return Err(DictionaryCodeError::Invalid {
                value: trimmed.to_string(),
            });
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DictionaryCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for DictionaryCode {
    type Err = DictionaryCodeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<&str> for DictionaryCode {
    type Error = DictionaryCodeError;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for DictionaryCode {
    type Error = DictionaryCodeError;

    #[inline]
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(&value)
    }
}

impl TryFrom<&String> for DictionaryCode {
    type Error = DictionaryCodeError;

    #[inline]
    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::new(value.as_str())
    }
}
