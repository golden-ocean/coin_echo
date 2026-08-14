use crate::error::{ErrorKind, ErrorMeta, FieldError};
use std::fmt;

/// 版本号解析/转换错误
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VersionMetaError {
    #[error("版本号不能为负数: {value}")]
    Invalid { value: i64 },
}

impl ErrorMeta for VersionMetaError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Validation
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Invalid { .. } => "version.invalid",
        }
    }

    fn detail(&self) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            Self::Invalid { value } => {
                Some(format!("版本号非法: {value}，版本号必须大于或等于 0").into())
            }
        }
    }

    fn fields(&self) -> Vec<FieldError> {
        match self {
            Self::Invalid { .. } => {
                vec![FieldError::new("version", "out_of_range")]
            }
        }
    }
}

/// 乐观锁版本号
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VersionMeta(i64);

impl VersionMeta {
    /// 构造默认的初始版本 (0)
    pub const fn new() -> Self {
        Self(0)
    }

    /// 获取自增后的下一版本
    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }

    /// 获取底层 i64 原始值
    pub const fn value(&self) -> i64 {
        self.0
    }

    /// 校验当前版本与预期版本一致（乐观锁更新前置判断）
    pub fn matches(&self, expect: i64) -> bool {
        self.0 == expect
    }
}

impl Default for VersionMeta {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for VersionMeta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<i64> for VersionMeta {
    type Error = VersionMetaError;

    fn try_from(v: i64) -> Result<Self, Self::Error> {
        if v >= 0 {
            Ok(Self(v))
        } else {
            Err(VersionMetaError::Invalid { value: v })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试初始版本为0，next 自增逻辑与不可变特性
    #[test]
    fn test_version_next_immutable() {
        let v0 = VersionMeta::new();
        assert_eq!(v0.value(), 0);

        let v1 = v0.next();
        assert_eq!(v1.value(), 1);

        let v2 = v1.next();
        assert_eq!(v2.value(), 2);

        // 原实例保持不变
        assert_eq!(v0.value(), 0);
    }

    /// 测试 TryFrom 校验机制
    #[test]
    fn test_version_try_from() {
        // 合法版本号转换
        assert_eq!(VersionMeta::try_from(0), Ok(VersionMeta(0)));
        assert_eq!(VersionMeta::try_from(100), Ok(VersionMeta(100)));

        // 负数版本号转换失败
        let err = VersionMeta::try_from(-1).unwrap_err();
        assert_eq!(err, VersionMetaError::Invalid { value: -1 });
    }

    /// 测试乐观锁 matches 匹配机制
    #[test]
    fn test_version_matches_expect() {
        let ver = VersionMeta::new().next().next(); // 2
        assert_eq!(ver.value(), 2);

        assert!(ver.matches(2));
        assert!(!ver.matches(99));
    }

    /// 测试 Ord 排序与比较
    #[test]
    fn test_version_ord_compare() {
        let v0 = VersionMeta::new();
        let v1 = v0.next();
        let v2 = v1.next();

        assert!(v0 < v1);
        assert!(v1 < v2);
        assert!(v2 > v0);
        assert_eq!(v0, VersionMeta::default());
    }

    /// 测试 Display 格式化输出
    #[test]
    fn test_version_display() {
        let v = VersionMeta::try_from(42).unwrap();
        assert_eq!(v.to_string(), "42");
    }

    /// 测试 ErrorMeta 特性接入
    #[test]
    fn test_version_error_meta() {
        let err = VersionMetaError::Invalid { value: -10 };

        assert_eq!(err.kind(), ErrorKind::Validation);
        assert_eq!(err.code(), "version.invalid");
        assert!(err.detail().unwrap().contains("-10"));

        let fields = err.fields();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].field.as_ref(), "version");
        assert_eq!(fields[0].code, "out_of_range");
    }
}
