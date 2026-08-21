use std::fmt;
use std::str::FromStr;

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DictionaryItemColorError {
    #[error("颜色值不能为空")]
    Empty,

    #[error("颜色值长度非法：{len}，应为 #RGB（4位）或 #RRGGBB（7位）格式")]
    InvalidLength { len: usize },

    #[error("颜色值格式非法：{value}，必须以 # 开头，其余部分仅允许十六进制字符")]
    Invalid { value: String },
}

impl ErrorMeta for DictionaryItemColorError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Validation
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Empty => "sys.dictionary_item.color.empty",
            Self::InvalidLength { .. } => "sys.dictionary_item.color.invalid_length",
            Self::Invalid { .. } => "sys.dictionary_item.color.invalid",
        }
    }

    fn fields(&self) -> Vec<FieldError> {
        vec![FieldError::new("color", self.code())]
    }
}

/// 前端展示用十六进制色值，如 #1890ff / #f00
/// 内部统一存储为小写，方便前端/仓储层直接比较与展示
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictionaryItemColor(String);

impl DictionaryItemColor {
    pub fn new(value: impl Into<String>) -> Result<Self, DictionaryItemColorError> {
        let s = value.into();
        let trimmed = s.trim();

        if trimmed.is_empty() {
            return Err(DictionaryItemColorError::Empty);
        }

        if !matches!(trimmed.len(), 4 | 7) {
            return Err(DictionaryItemColorError::InvalidLength { len: trimmed.len() });
        }

        let valid = trimmed.starts_with('#') && trimmed[1..].chars().all(|c| c.is_ascii_hexdigit());

        if !valid {
            return Err(DictionaryItemColorError::Invalid {
                value: trimmed.to_string(),
            });
        }

        Ok(Self(trimmed.to_lowercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DictionaryItemColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for DictionaryItemColor {
    type Err = DictionaryItemColorError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<&str> for DictionaryItemColor {
    type Error = DictionaryItemColorError;
    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for DictionaryItemColor {
    type Error = DictionaryItemColorError;
    #[inline]
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(&value)
    }
}

impl TryFrom<&String> for DictionaryItemColor {
    type Error = DictionaryItemColorError;
    #[inline]
    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::new(value.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_short_and_long_hex() {
        assert!(DictionaryItemColor::new("#fff").is_ok());
        assert!(DictionaryItemColor::new("#1890FF").is_ok());
    }

    #[test]
    fn normalizes_to_lowercase() {
        let color = DictionaryItemColor::new("#1890FF").unwrap();
        assert_eq!(color.as_str(), "#1890ff");
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(
            DictionaryItemColor::new(""),
            Err(DictionaryItemColorError::Empty)
        );
        assert_eq!(
            DictionaryItemColor::new("   "),
            Err(DictionaryItemColorError::Empty)
        );
    }

    #[test]
    fn rejects_missing_hash() {
        let err = DictionaryItemColor::new("1890ffx").unwrap_err();
        assert!(matches!(err, DictionaryItemColorError::Invalid { .. }));
    }

    #[test]
    fn rejects_non_hex_chars() {
        let err = DictionaryItemColor::new("#zzzzzz").unwrap_err();
        assert!(matches!(err, DictionaryItemColorError::Invalid { .. }));
    }

    #[test]
    fn rejects_wrong_length() {
        let err = DictionaryItemColor::new("#12345").unwrap_err();
        assert!(matches!(
            err,
            DictionaryItemColorError::InvalidLength { len: 6 }
        ));
    }

    #[test]
    fn from_str_and_try_from_are_consistent() {
        let a: DictionaryItemColor = "#abc".parse().unwrap();
        let b = DictionaryItemColor::try_from("#abc").unwrap();
        let c = DictionaryItemColor::try_from("#abc".to_string()).unwrap();
        assert_eq!(a, b);
        assert_eq!(b, c);
    }
}
