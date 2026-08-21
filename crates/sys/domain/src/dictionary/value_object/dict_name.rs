use std::fmt;
use std::str::FromStr;

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DictionaryNameError {
    #[error("字典名称不能为空")]
    Empty,
    #[error("字典名称过长：{len}，最大允许 64 个字符")]
    TooLong { len: usize },
}

impl ErrorMeta for DictionaryNameError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Validation
    }
    fn code(&self) -> &'static str {
        match self {
            Self::Empty => "sys.dictionary.code.empty",
            Self::TooLong { .. } => "sys.dictionary.code.too_long",
        }
    }
    fn fields(&self) -> Vec<FieldError> {
        vec![FieldError::new("code", self.code())]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictionaryName(String);

impl DictionaryName {
    pub fn new(value: impl Into<String>) -> Result<Self, DictionaryNameError> {
        let s = value.into();
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(DictionaryNameError::Empty);
        }
        if trimmed.len() > 64 {
            return Err(DictionaryNameError::TooLong { len: trimmed.len() });
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DictionaryName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for DictionaryName {
    type Err = DictionaryNameError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<&str> for DictionaryName {
    type Error = DictionaryNameError;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for DictionaryName {
    type Error = DictionaryNameError;

    #[inline]
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(&value)
    }
}

impl TryFrom<&String> for DictionaryName {
    type Error = DictionaryNameError;

    #[inline]
    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::new(value.as_str())
    }
}
