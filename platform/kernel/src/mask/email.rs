use crate::mask::redact::{MASK_STAR, REDACT};
use crate::mask::utils::char_prefix;

/// 邮箱脱敏处理
/// 脱敏规则：
/// 1. 非标准邮箱（无@、多@、用户名/域名为空）统一返回 **
/// 2. 用户名字符数 ≤ 2：全部掩码，格式 `**@域名`
/// 3. 用户名字符数 > 2：保留首字符，中间掩码，格式 `首字符**@域名`
/// # Arguments
/// * `email` - 原始邮箱字符串
/// # Returns
/// 脱敏后的邮箱字符串，非法邮箱固定返回 "**"
pub fn mask_email(email: &str) -> String {
    // 去除首尾空白字符，避免空格干扰校验
    let email_trim = email.trim();

    // 必须且只能包含一个@符号
    let Some((name, domain)) = email_trim.split_once('@') else {
        return REDACT.to_string();
    };
    if domain.contains('@') {
        return REDACT.to_string();
    }

    // 用户名、域名不能为空
    let name = name.trim();
    let domain = domain.trim();
    if name.is_empty() || domain.is_empty() {
        return REDACT.to_string();
    }

    let name_len = name.chars().count();

    // 用户名长度≤2，完整隐藏用户名
    if name_len <= 2 {
        return format!("{MASK_STAR}@{domain}");
    }

    // 用户名长度>2，保留第一个字符，中间统一掩码
    format!("{}{MASK_STAR}@{}", char_prefix(name, 1), domain)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_email_all_cases() {
        // ========== 场景1：合法邮箱 - 用户名字符数 <=2，全部掩码 ==========
        let email1 = "a@qq.com";
        assert_eq!(mask_email(email1), "***@qq.com");
        let email2 = "ab@163.com";
        assert_eq!(mask_email(email2), "***@163.com");
        let email3 = "张@outlook.com";
        assert_eq!(mask_email(email3), "***@outlook.com");
        let email4 = "张三@gmail.com";
        assert_eq!(mask_email(email4), "***@gmail.com");

        // ========== 场景2：合法邮箱 - 用户名字符数 >2，保留首字符 ==========
        let email5 = "abc@qq.com";
        assert_eq!(mask_email(email5), "a***@qq.com");
        let email6 = "jackson123@company.org";
        assert_eq!(mask_email(email6), "j***@company.org");
        let email7 = "李four@edu.cn";
        assert_eq!(mask_email(email7), "李***@edu.cn");
        let email8 = "test_001@foxmail.com";
        assert_eq!(mask_email(email8), "t***@foxmail.com");

        // ========== 场景3：带首尾空白的邮箱（自动修剪空格后正常脱敏） ==========
        let email9 = "  lily@126.com";
        assert_eq!(mask_email(email9), "l***@126.com");
        let email10 = "wang@aliyun.com  ";
        assert_eq!(mask_email(email10), "w***@aliyun.com");
        let email11 = "  zhao666@vip.qq.com  ";
        assert_eq!(mask_email(email11), "z***@vip.qq.com");

        // ========== 场景4：非法邮箱，统一返回 redact() "[REDACTED]" ==========
        let email12 = "123456789";
        assert_eq!(mask_email(email12), REDACT.to_string());
        let email13 = "@";
        assert_eq!(mask_email(email13), REDACT.to_string());
        let email14 = "@baidu.com";
        assert_eq!(mask_email(email14), REDACT.to_string());
        let email15 = "test@";
        assert_eq!(mask_email(email15), REDACT.to_string());
        let email16 = "user@name@qq.com";
        assert_eq!(mask_email(email16), REDACT.to_string());
        let email17 = "";
        assert_eq!(mask_email(email17), REDACT.to_string());
        let email18 = "   ";
        assert_eq!(mask_email(email18), REDACT.to_string());
        let email19 = "   @   ";
        assert_eq!(mask_email(email19), REDACT.to_string());
    }
}
