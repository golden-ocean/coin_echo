use crate::mask::redact::{MASK_STAR, REDACT};
use crate::mask::utils::{char_prefix, char_suffix};

/// 通用密钥/隐私短字符串脱敏
/// 规则：
/// 1. 自动修剪首尾空白字符
/// 2. 修剪后字符总数 ≤6：整体返回全脱敏 [REDACTED]
/// 3. 字符总数 >6：保留前3字符 + 中间***遮挡 + 末尾3字符
///
/// # Examples
/// ```
/// use platform_kernel::mask::mask_secret;
/// assert_eq!(mask_secret("abcdef123456"), "abc***456");
/// assert_eq!(mask_secret("123456"), "[REDACTED]");
/// ```
pub fn mask_secret(value: &str) -> String {
    // 剔除首尾空白，空格不参与有效字符统计
    let raw = value.trim();
    let char_count = raw.chars().count();

    if char_count <= 6 {
        return REDACT.to_string();
    }

    format!(
        "{}{}{}",
        char_prefix(raw, 3),
        MASK_STAR,
        char_suffix(raw, 3)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mask::redact::{MASK_STAR, REDACT};

    #[test]
    fn test_mask_secret_all_cases() {
        // ========== 场景1：有效长度>6，标准脱敏展示 ==========
        // 英文密钥
        let s1 = "abcdef123456";
        assert_eq!(mask_secret(s1), format!("abc{MASK_STAR}456"));
        // 纯数字长密钥
        let s2 = "9876543210";
        assert_eq!(mask_secret(s2), format!("987{MASK_STAR}210"));
        // 中文密钥（UTF-8多字节兼容）
        let s3 = "一二三四五六七八九十";
        assert_eq!(mask_secret(s3), format!("一二三{MASK_STAR}八九十"));

        // ========== 场景2：临界长度 7（刚好大于6，触发脱敏） ==========
        let s4 = "1234567";
        assert_eq!(mask_secret(s4), format!("123{MASK_STAR}567"));

        // ========== 场景3：长度≤6，统一返回全局全脱敏占位 ==========
        // 刚好6字符临界
        let s5 = "abc123";
        assert_eq!(mask_secret(s5), REDACT.to_string());
        // 3位短串
        let s6 = "xyz";
        assert_eq!(mask_secret(s6), REDACT.to_string());
        // 单个字符
        let s7 = "7";
        assert_eq!(mask_secret(s7), REDACT.to_string());
        // 空字符串输入
        let s8 = "";
        assert_eq!(mask_secret(s8), REDACT.to_string());
        // 全空白字符（trim后为空）
        let s9 = "   ";
        assert_eq!(mask_secret(s9), REDACT.to_string());

        // ========== 场景4：首尾带空白，自动修剪后正常处理 ==========
        // 左侧空格
        let s10 = "  secretkey123";
        assert_eq!(mask_secret(s10), format!("sec{MASK_STAR}123"));
        // 右侧空格
        let s11 = "abc123456789  ";
        assert_eq!(mask_secret(s11), format!("abc{MASK_STAR}789"));
        // 前后包裹大量空格
        let s12 = "    测试密钥888888    ";
        assert_eq!(mask_secret(s12), format!("测试密{MASK_STAR}888"));

        // ========== 场景5：带特殊符号密钥 ==========
        let s13 = "A1_@#z98765";
        assert_eq!(mask_secret(s13), format!("A1_{MASK_STAR}765"));
    }

    /// 配套常量校验，保证脱敏符号统一
    #[test]
    fn test_mask_constants() {
        assert_eq!(REDACT, "[REDACTED]");
        assert_eq!(MASK_STAR, "***");
    }
}
