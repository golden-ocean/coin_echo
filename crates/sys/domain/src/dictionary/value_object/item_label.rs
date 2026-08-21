use std::fmt;
use std::str::FromStr;

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DictionaryItemLabelError {
    #[error("字典项标签不能为空")]
    Empty,
    #[error("字典项标签过长：{len} 个字符，最大允许 100 个字符")]
    TooLong { len: usize },
}

impl ErrorMeta for DictionaryItemLabelError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Validation
    }
    fn code(&self) -> &'static str {
        match self {
            Self::Empty => "sys.dictionary_item.label.empty",
            Self::TooLong { .. } => "sys.dictionary_item.label.too_long",
        }
    }
    fn fields(&self) -> Vec<FieldError> {
        vec![FieldError::new("label", self.code())]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictionaryItemLabel(String);

impl DictionaryItemLabel {
    pub fn new(value: impl Into<String>) -> Result<Self, DictionaryItemLabelError> {
        let s = value.into();
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(DictionaryItemLabelError::Empty);
        }
        // 注意：字符长度校验使用 chars().count() 统计 Unicode 字符数，而非字节数 len()
        let char_count = trimmed.chars().count();
        if char_count > 100 {
            return Err(DictionaryItemLabelError::TooLong { len: char_count });
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DictionaryItemLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for DictionaryItemLabel {
    type Err = DictionaryItemLabelError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<&str> for DictionaryItemLabel {
    type Error = DictionaryItemLabelError;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for DictionaryItemLabel {
    type Error = DictionaryItemLabelError;

    #[inline]
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&String> for DictionaryItemLabel {
    type Error = DictionaryItemLabelError;

    #[inline]
    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::new(value.as_str())
    }
}
