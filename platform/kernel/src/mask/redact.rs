use std::fmt;

/// 全局完整脱敏占位文本常量
pub const REDACT: &str = "[REDACTED]";
pub const MASK_STAR: &str = "***";

/// 零开销包装类型，包裹敏感数据，屏蔽 Debug/Display 明文输出
/// 仅在打印日志、格式化时脱敏，.inner()/.into_inner() 可取出原始值用于业务处理
///
/// # Examples
/// ```
/// use platform_kernel::mask::{Redacted, REDACT};
///
/// let raw_email = "test@example.com";
/// let wrapped = Redacted::new(raw_email);
///
/// // 打印脱敏，不会泄露明文
/// assert_eq!(format!("{}", wrapped), REDACT);
/// assert_eq!(format!("{:?}", wrapped), REDACT);
///
/// // 业务取原始明文
/// assert_eq!(wrapped.inner(), &raw_email);
/// let inner = wrapped.into_inner();
/// assert_eq!(inner, raw_email);
/// ```
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Redacted<T>(pub T);

impl<T> Redacted<T> {
    /// 构造脱敏包装器
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// 获取内部原始值引用（业务逻辑读取明文使用）
    pub fn inner(&self) -> &T {
        &self.0
    }

    /// 消耗包装器，取出所有权原始值
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> fmt::Debug for Redacted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACT)
    }
}

impl<T> fmt::Display for Redacted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACT)
    }
}

impl<T> From<T> for Redacted<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_constant() {
        assert_eq!(REDACT, "[REDACTED]");
    }

    #[test]
    fn test_redacted_wrap_unwrap() {
        let origin = "13800001234";
        let wrap1 = Redacted::new(origin);
        let wrap2: Redacted<&str> = origin.into();

        assert_eq!(wrap1, wrap2);
        assert_eq!(wrap1.inner(), &origin);
        let inner = wrap1.into_inner();
        assert_eq!(inner, origin);
    }

    #[test]
    fn test_redacted_display() {
        let val = Redacted::new("secret@mail.com");
        let show = format!("{}", val);
        assert_eq!(show, REDACT);
    }

    #[test]
    fn test_redacted_debug() {
        let val = Redacted::new("123456789");
        let debug_str = format!("{:?}", val);
        assert_eq!(debug_str, REDACT);
    }

    #[test]
    fn test_redacted_clone_copy() {
        let base = Redacted::new(666);
        let copied = base;
        let cloned = base.clone();
        assert_eq!(copied.into_inner(), 666);
        assert_eq!(cloned.into_inner(), 666);
    }
}
