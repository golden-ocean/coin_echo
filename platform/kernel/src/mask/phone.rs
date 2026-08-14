use crate::mask::redact::REDACT;
use crate::mask::utils::{char_prefix, char_suffix};

/// 手机号脱敏处理
/// 脱敏规则：
/// 1. 去除首尾空白字符后校验
/// 2. 有效字符总数不足7位：整体返回 [REDACTED]
/// 3. 字符≥7位：保留前3位 + 中间4星 + 末尾4位
/// 示例：13800001234 → 138****1234
/// # Examples
/// ```
/// use platform_kernel::mask::mask_phone;
/// assert_eq!(mask_phone("135123"), "[REDACTED]");
/// assert_eq!(mask_phone("13800001234"), "138****1234");
/// ```
pub fn mask_phone(phone: &str) -> String {
    // 剔除首尾空格，避免空白干扰长度截取
    let raw = phone.trim();
    let char_cnt = raw.chars().count();

    // 总字符不足7位，直接全脱敏
    if char_cnt < 7 {
        return REDACT.to_string();
    }

    format!("{}****{}", char_prefix(raw, 3), char_suffix(raw, 4))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mask::redact::REDACT;

    #[test]
    fn test_mask_phone_all_scenes() {
        // ========== 场景1：标准11位大陆手机号（正常脱敏核心用例） ==========
        // 标准11位数字
        let p1 = "13800001234";
        assert_eq!(mask_phone(p1), "138****1234");

        // ========== 场景2：刚好7位临界长度（最低展示脱敏阈值） ==========
        // 7位：前3 + **** + 后4，完整拼接
        let p2 = "1234567";
        assert_eq!(mask_phone(p2), "123****4567");

        // ========== 场景3：超过11位长号码，仅保留前3后4 ==========
        let p3 = "13988889999123";
        assert_eq!(mask_phone(p3), "139****9123");

        // ========== 场景4：首尾带空白，自动trim后正常脱敏 ==========
        // 左侧空格
        let p4 = " 13711223344";
        assert_eq!(mask_phone(p4), "137****3344");
        // 右侧空格
        let p5 = "13655667788  ";
        assert_eq!(mask_phone(p5), "136****7788");
        // 前后全空格包裹
        let p6 = "  13501010202  ";
        assert_eq!(mask_phone(p6), "135****0202");

        // ========== 场景5：字符不足7位，统一返回全局脱敏占位符 ==========
        // 6位数字
        let p7 = "123456";
        assert_eq!(mask_phone(p7), REDACT.to_string());
        // 1位数字
        let p8 = "9";
        assert_eq!(mask_phone(p8), REDACT.to_string());
        // 纯空格字符串（trim后为空，长度0）
        let p9 = "    ";
        assert_eq!(mask_phone(p9), REDACT.to_string());
        // 空字符串输入
        let p10 = "";
        assert_eq!(mask_phone(p10), REDACT.to_string());

        // ========== 场景6：包含特殊符号/中文混合，仍按字符长度脱敏 ==========
        // 带分隔符短号（长度足够会脱敏）
        let p11 = "138-1234-5678";
        assert_eq!(mask_phone(p11), "138****5678");
        // 中英混合手机号
        let p12 = "139abc12345";
        assert_eq!(mask_phone(p12), "139****2345");
        // 中文+数字组合，长度达标
        let p13 = "张13600001111";
        assert_eq!(mask_phone(p13), "张13****1111");
    }
}
